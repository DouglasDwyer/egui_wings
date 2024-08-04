use fxhash::*;
use serde::*;
use serde::de::*;
use std::hash::*;
use std::sync::*;

pub struct ArcCache<T> {
    arc_to_id: FxHashMap<HashByArc<T>, u64>,
    cache_delta: ArcCacheDelta<T>,
    id_counter: u64,
    id_to_arc: FxHashMap<u64, Arc<T>>,
}

impl<T> ArcCache<T> {
    pub fn apply_delta(&mut self, delta: ArcCacheDelta<T>) {
        self.id_counter = delta.id_counter;
        for (id, value) in delta.to_add {
            self.id_to_arc.insert(id, value.clone());
            self.arc_to_id.insert(HashByArc(value), id);
        }

        self.cache_delta.to_add.clear();
        self.cache_delta.to_remove.clear();
    }

    pub fn get_by_id(&self, id: u64) -> Option<&Arc<T>> {
        self.id_to_arc.get(&id)
    }

    pub fn load_delta(&mut self) -> &ArcCacheDelta<T> {
        for (to_remove, _) in self.id_to_arc.iter().filter(|(_, x)| Arc::strong_count(x) == 2) {
            self.cache_delta.to_remove.push(*to_remove);
        }

        for id in &self.cache_delta.to_remove {
            let ptr = self.id_to_arc.remove(id).expect("Failed to get item to remove.");
            self.arc_to_id.remove(&HashByArc(ptr));
        }

        &self.cache_delta
    }

    pub fn insert(&mut self, value: Arc<T>) -> u64 {
        match self.arc_to_id.entry(HashByArc(value.clone())) {
            std::collections::hash_map::Entry::Occupied(x) => *x.get(),
            std::collections::hash_map::Entry::Vacant(x) => {
                let id = self.id_counter;
                self.id_counter += 1;
                x.insert(id);
                self.id_to_arc.insert(id, value.clone());
                self.cache_delta.to_add.push((id, value.clone()));
                id
            }
        }
    }
}

pub struct ArcCacheDelta<T> {
    id_counter: u64,
    to_add: Vec<(u64, Arc<T>)>,
    to_remove: Vec<u64>,
}


#[derive(Clone)]
struct HashByArc<T>(pub Arc<T>);

impl<T> Hash for HashByArc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

impl<T> PartialEq for HashByArc<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for HashByArc<T> {}