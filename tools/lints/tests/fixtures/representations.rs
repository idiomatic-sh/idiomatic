pub mod interface {
    use std::collections::{
        BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque,
    };

    pub trait ExposeInterface {
        fn flag(&self) -> bool;
        fn letter(&self) -> char;
        fn text(&self) -> &str;
        fn tiny(&self) -> i8;
        fn small(&self) -> i16;
        fn medium(&self) -> i32;
        fn large(&self) -> i64;
        fn huge(&self) -> i128;
        fn signed(&self) -> isize;
        fn byte(&self) -> u8;
        fn short(&self) -> u16;
        fn word(&self) -> u32;
        fn long(&self) -> u64;
        fn wide(&self) -> u128;
        fn index(&self) -> usize;
        fn single(&self) -> f32;
        fn double(&self) -> f64;
        fn owned(&self) -> String;
        fn listed(&self) -> Vec<foreign::Count>;
        fn queued(&self) -> VecDeque<foreign::Count>;
        fn linked(&self) -> LinkedList<foreign::Count>;
        fn hashed(&self) -> HashMap<foreign::Count, foreign::Count>;
        fn set(&self) -> HashSet<foreign::Count>;
        fn sorted(&self) -> BTreeMap<foreign::Count, foreign::Count>;
        fn ordered(&self) -> BTreeSet<foreign::Count>;
        fn heap(&self) -> BinaryHeap<foreign::Count>;
        fn maybe(&self) -> Option<foreign::Count>;
    }
}
