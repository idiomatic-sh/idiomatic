//! The `follow` service: the one meaning of `follow` over one key group's edges. A pure function: no IO and no engine type.

use super::types::{Arrival, Entity, GroupPointers, Pointer, SourcePlace};

/// `follow`: appends to `out` each pair of a source and a fixed point it reaches over the edges of `group`, each pair once, in
/// source order and then by fixed point. A fixed point is an entity with an edge to itself, and it answers itself. A walk that runs
/// out of edges answers no row, and neither does a cycle longer than one edge that reaches no fixed point; a source with several
/// targets answers every fixed point it reaches. There is no depth bound: each fixed point is walked back from once, so the work is
/// the edges, sorted, when each source has one target, and the edges times the fixed points at most.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::follow::service::fixed_points;
/// use ryo_operators::domain::follow::types::{Entity, GroupPointers, Pointer};
/// let edge = |source, destination| Pointer { source: Entity(source), destination: Entity(destination) };
/// let edges = [edge(1, 2), edge(2, 3), edge(3, 3), edge(4, 5)];
/// let mut out = Vec::new();
/// fixed_points(GroupPointers(&edges), &mut out);
/// assert_eq!(out, [edge(1, 3), edge(2, 3), edge(3, 3)]);
/// ```
pub fn fixed_points(group: GroupPointers<'_>, out: &mut Vec<Pointer>) {
    let answered = out.len();
    // The entities an edge leaves, in order: every fixed point, and everything that reaches one, is among them.
    let mut sources: Vec<Entity> = group.0.iter().map(|edge| edge.source).collect();
    sources.dedup();
    let place_of = |entity: Entity| {
        let at = sources.partition_point(|source| *source < entity);
        (sources.get(at) == Some(&entity)).then_some(SourcePlace(at))
    };
    // The edges between those entities turned round, by where they arrive: a walk back from a fixed point reads them.
    let mut arrivals: Vec<Arrival> = group
        .0
        .iter()
        .filter_map(|edge| {
            Some(Arrival {
                at: place_of(edge.destination)?,
                from: place_of(edge.source)?,
            })
        })
        .collect();
    arrivals.sort_unstable();
    arrivals.dedup();
    // The fixed point whose walk back last reached each entity: an entity is answered once for each fixed point.
    let mut reached: Vec<Option<SourcePlace>> = vec![None; sources.len()];
    let mut pending: Vec<SourcePlace> = Vec::new();
    for (fixed, end) in sources.iter().enumerate() {
        let fixed = SourcePlace(fixed);
        let own = Arrival {
            at: fixed,
            from: fixed,
        };
        if arrivals.binary_search(&own).is_err() {
            continue;
        }
        if let Some(mark) = reached.get_mut(fixed.0) {
            *mark = Some(fixed);
        }
        pending.push(fixed);
        while let Some(node) = pending.pop() {
            if let Some(source) = sources.get(node.0) {
                out.push(Pointer {
                    source: *source,
                    destination: *end,
                });
            }
            let (_, arriving) =
                arrivals.split_at(arrivals.partition_point(|arrival| arrival.at < node));
            for arrival in arriving.iter().take_while(|arrival| arrival.at == node) {
                if let Some(mark) = reached.get_mut(arrival.from.0)
                    && *mark != Some(fixed)
                {
                    *mark = Some(fixed);
                    pending.push(arrival.from);
                }
            }
        }
    }
    if let Some(added) = out.get_mut(answered..) {
        added.sort_unstable();
    }
}
