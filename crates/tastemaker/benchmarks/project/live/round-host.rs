// Installed only in the benchmark copy of Tastemaker.
use crate::outbound::analysis::{
    errors::AnalysisError,
    interface::AnalysisPort,
    types::{AnalysisReport, AnalysisRequest},
};
use std::{
    cell::RefCell,
    future::{Future, poll_fn},
    rc::Rc,
    task::{Context, Poll, Waker},
};

type Answer = Result<AnalysisReport, AnalysisError>;
type Delivery = Rc<RefCell<Option<Answer>>>;
type Queue = Vec<(AnalysisRequest, Delivery)>;

thread_local! {
    static REQUESTS: RefCell<Option<Queue>> = const { RefCell::new(None) };
}

pub(super) fn enabled() -> bool {
    std::env::var_os("TM_BENCH_ROUNDS").is_some()
}

pub(super) async fn lint(analysis: &dyn AnalysisPort, request: AnalysisRequest) -> Answer {
    let mut request = Some(request);
    let delivery = REQUESTS.with(|queue| {
        let mut queue = queue.borrow_mut();
        queue.as_mut().map(|queue| {
            let delivery = Delivery::default();
            queue.push((request.take().unwrap(), delivery.clone()));
            delivery
        })
    });
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
    let mut pending: Vec<_> = futures
        .into_iter()
        .map(|future| (Some(Box::pin(future)), None))
        .collect();
    poll_fn(|context| {
        let mut waiting = false;
        for (future, value) in &mut pending {
            if let Some(next) = future {
                match next.as_mut().poll(context) {
                    Poll::Ready(out) => {
                        *value = Some(out);
                        *future = None;
                    }
                    Poll::Pending => waiting = true,
                }
            }
        }
        if waiting {
            Poll::Pending
        } else {
            Poll::Ready(
                pending
                    .iter_mut()
                    .map(|(_, result)| result.take().unwrap())
                    .collect(),
            )
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
            REQUESTS.with(|queue| {
                queue.borrow_mut().take();
            });
        }
    }
    let batched = enabled();
    let _reset = batched.then(|| {
        REQUESTS.with(|queue| assert!(queue.borrow_mut().replace(Vec::new()).is_none()));
        Reset
    });
    let mut proofs = std::pin::pin!(proofs);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        match proofs.as_mut().poll(&mut context) {
            Poll::Ready(out) => return out,
            Poll::Pending => {
                let work =
                    REQUESTS.with(|queue| std::mem::take(queue.borrow_mut().as_mut().unwrap()));
                assert!(
                    !work.is_empty(),
                    "only a queued lint answer can suspend a proof"
                );
                let (requests, deliveries): (Vec<_>, Vec<_>) = work.into_iter().unzip();
                let answers = analysis.lint_benchmark_round(requests);
                assert_eq!(answers.len(), deliveries.len());
                for (delivery, answer) in deliveries.into_iter().zip(answers) {
                    *delivery.borrow_mut() = Some(answer);
                }
            }
        }
    }
}
