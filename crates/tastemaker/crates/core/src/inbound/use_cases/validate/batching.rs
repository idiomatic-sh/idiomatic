//! The ordered batching of the proofs `validate` runs: each proof is a future that suspends where it asks the analysis port for a lint,
//! the driver collects the requests of up to `batch_capacity` pending proofs, sends them as ONE ordered batch and wakes each proof with its
//! answer, in the order of the requests. With a capacity of one the calls go through `lint` one at a time, as before.
use crate::outbound::analysis::{
    errors::AnalysisError,
    interface::AnalysisPort,
    types::{AnalysisReport, AnalysisRequest},
};
use std::{
    cell::RefCell,
    future::poll_fn,
    rc::Rc,
    task::{Context, Poll, Waker},
};

type Answer = Result<AnalysisReport, AnalysisError>;
type Delivery = Rc<RefCell<Option<Answer>>>;

struct Ready {
    calls: Vec<(AnalysisRequest, Delivery)>,
    capacity: usize,
}

thread_local! {
    static REQUESTS: RefCell<Option<Ready>> = const { RefCell::new(None) };
}

/// Whether the proofs being driven are batched: a queue is installed only when the analysis port takes more than one call at a time.
pub(super) fn enabled() -> bool {
    REQUESTS.with(|queue| queue.borrow().is_some())
}

fn may_queue() -> bool {
    REQUESTS.with(|queue| {
        queue
            .borrow()
            .as_ref()
            .is_none_or(|ready| ready.calls.len() < ready.capacity)
    })
}

pub(super) async fn lint(analysis: &dyn AnalysisPort, request: AnalysisRequest) -> Answer {
    let mut request = Some(request);
    let delivery = poll_fn(|_| {
        REQUESTS.with(|queue| {
            let mut queue = queue.borrow_mut();
            let Some(ready) = queue.as_mut() else {
                return Poll::Ready(None);
            };
            if ready.calls.len() == ready.capacity {
                return Poll::Pending;
            }
            let delivery = Delivery::default();
            ready
                .calls
                .push((request.take().unwrap(), delivery.clone()));
            Poll::Ready(Some(delivery))
        })
    })
    .await;
    match delivery {
        None => analysis.lint(request.unwrap()),
        Some(delivery) => {
            poll_fn(|_| match delivery.borrow_mut().take() {
                Some(answer) => Poll::Ready(answer),
                None => Poll::Pending,
            })
            .await
        }
    }
}

pub(super) async fn ordered<F: Future>(futures: impl IntoIterator<Item = F>) -> Vec<F::Output> {
    let mut waiting = futures.into_iter().fuse();
    let mut pending = Vec::<(usize, std::pin::Pin<Box<F>>)>::new();
    let mut outputs = Vec::<Option<F::Output>>::new();
    poll_fn(|context| {
        let mut remaining = Vec::new();
        for (index, mut future) in pending.drain(..) {
            if may_queue()
                && let Poll::Ready(value) = future.as_mut().poll(context)
            {
                outputs[index] = Some(value);
            } else {
                remaining.push((index, future));
            }
        }
        let mut finished = false;
        while may_queue() {
            let Some(next) = waiting.next() else {
                finished = true;
                break;
            };
            let index = outputs.len();
            let mut next = Box::pin(next);
            match next.as_mut().poll(context) {
                Poll::Ready(value) => outputs.push(Some(value)),
                Poll::Pending => {
                    outputs.push(None);
                    remaining.push((index, next));
                }
            }
        }
        pending = remaining;
        if finished && pending.is_empty() {
            Poll::Ready(outputs.drain(..).map(Option::unwrap).collect())
        } else {
            Poll::Pending
        }
    })
    .await
}

pub(super) async fn together<A: Future, B: Future>(left: A, right: B) -> (A::Output, B::Output) {
    let mut left = std::pin::pin!(left);
    let mut right = std::pin::pin!(right);
    let (mut a, mut b) = (None, None);
    poll_fn(|cx| {
        if a.is_none()
            && let Poll::Ready(out) = left.as_mut().poll(cx)
        {
            a = Some(out);
        }
        if b.is_none()
            && may_queue()
            && let Poll::Ready(out) = right.as_mut().poll(cx)
        {
            b = Some(out);
        }
        if a.is_some() && b.is_some() {
            Poll::Ready((a.take().unwrap(), b.take().unwrap()))
        } else {
            Poll::Pending
        }
    })
    .await
}

pub(super) fn drive<F: Future>(analysis: &dyn AnalysisPort, proofs: F) -> F::Output {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            REQUESTS.with(|queue| drop(queue.borrow_mut().take()));
        }
    }
    let capacity = analysis.batch_capacity();
    let _reset = (capacity > 1).then(|| {
        REQUESTS.with(|queue| {
            assert!(
                queue
                    .borrow_mut()
                    .replace(Ready {
                        calls: Vec::with_capacity(capacity),
                        capacity,
                    })
                    .is_none()
            )
        });
        Reset
    });
    let mut proofs = std::pin::pin!(proofs);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        match proofs.as_mut().poll(&mut context) {
            Poll::Ready(out) => return out,
            Poll::Pending => {
                let (requests, deliveries): (Vec<_>, Vec<_>) = REQUESTS
                    .with(|queue| queue.borrow_mut().as_mut().unwrap().calls.drain(..).unzip());
                assert!(
                    !requests.is_empty(),
                    "only a queued lint answer can suspend a proof"
                );
                let answers = analysis.lint_batch(requests);
                assert_eq!(answers.len(), deliveries.len());
                for (delivery, answer) in deliveries.into_iter().zip(answers) {
                    *delivery.borrow_mut() = Some(answer);
                }
            }
        }
    }
}
