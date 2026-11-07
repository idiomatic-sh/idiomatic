//! Request-owned structural interning; source admission and proof interpretation stay at the caller.
use super::types::ProjectSourceMatch;
use std::rc::Rc;

/// Values remain private to the phase that supplies their keys and issues their identities.
pub(in crate::application::file_dag::program) struct Populations<S, K, I> {
    sources: Vec<Rc<S>>,
    invocations: Vec<ProjectInvocationEntry<K, I>>,
}
struct ProjectInvocationEntry<K, I> {
    key: K,
    identity: I,
}
impl<S, K, I> Default for Populations<S, K, I> {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            invocations: Vec::new(),
        }
    }
}
impl<S, K: Eq, I: Copy> Populations<S, K, I> {
    pub(in crate::application::file_dag::program) fn source<E>(
        &mut self,
        equivalent: impl Fn(&S) -> ProjectSourceMatch,
        issue: impl FnOnce() -> Result<S, E>,
    ) -> Result<Rc<S>, E> {
        if let Some(source) = self
            .sources
            .iter()
            .find(|source| matches!(equivalent(source), ProjectSourceMatch::Equivalent))
        {
            return Ok(Rc::clone(source));
        }
        let source = Rc::new(issue()?);
        self.sources.push(Rc::clone(&source));
        Ok(source)
    }
    pub(in crate::application::file_dag::program) fn invocation<E>(
        &mut self,
        key: K,
        issue: impl FnOnce() -> Result<I, E>,
    ) -> Result<I, E> {
        if let Some(entry) = self.invocations.iter().find(|entry| entry.key == key) {
            return Ok(entry.identity);
        }
        let identity = issue()?;
        self.invocations
            .push(ProjectInvocationEntry { key, identity });
        Ok(identity)
    }
}
