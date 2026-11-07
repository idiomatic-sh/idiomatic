pub struct Sequential;
pub struct Work<I>(pub Option<I>);
pub mod interface {
    pub trait ScheduleInterface {
        fn map<I, O, F: Fn(I) -> O>(&self, work: super::Work<I>, operation: F) -> super::Work<O>;
    }
}
impl interface::ScheduleInterface for Sequential {
    fn map<I, O, F: Fn(I) -> O>(&self, work: Work<I>, operation: F) -> Work<O> {
        Work(work.0.map(operation))
    }
}
