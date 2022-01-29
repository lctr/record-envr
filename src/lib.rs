use eithr::Either;

use std::{
    collections::{
        hash_map::Keys, HashMap, HashSet,
    },
    fmt::{self, Write},
};

/// Supertrait of traits necessary to be satisfied in order to be
/// held in a hashtable.
pub trait Hashley:
    std::cmp::Eq + std::hash::Hash
{
}

impl<T> Hashley for T where
    T: std::cmp::Eq + std::hash::Hash
{
}

#[derive(Clone, Default)]
pub struct Envr<K, V>(
    HashMap<K, V>,
    Option<Box<Self>>,
);

impl<K, V> Envr<K, V>
where
    K: std::cmp::Eq + std::hash::Hash,
{
    pub fn new() -> Envr<K, V> {
        Self(HashMap::default(), None)
    }

    /// Total number of entries stored. Note that this value is equal to
    /// the number of entries stored locally + the number of entries
    /// inherited
    pub fn size(&self) -> usize {
        let mut count = self.0.len();
        if let Some(p) = &self.1 {
            count += p.size();
        }
        count
    }

    pub fn new_from(
        parent: Option<Envr<K, V>>,
    ) -> Envr<K, V> {
        Self(HashMap::new(), parent.map(Box::new))
    }

    pub fn get_locals(&self) -> &HashMap<K, V> {
        &self.0
    }

    pub fn get_parent(
        &self,
    ) -> &Option<Box<Envr<K, V>>> {
        &self.1
    }

    /// Returns `true` if the environment has a parent environment, i.e.,
    /// if it is the extension of another environment.
    /// Otherwise returns `false`
    pub fn has_parent(&self) -> bool {
        self.1.is_some()
    }

    pub fn extend(self) -> Envr<K, V> {
        Self(HashMap::new(), Some(Box::new(self)))
    }

    /// Clones the environment and produces a new environment
    /// extended from the clone.
    pub fn extension(&self) -> Envr<K, V>
    where
        K: Clone,
        V: Clone,
    {
        Self(
            HashMap::new(),
            Some(Box::new(self.clone())),
        )
    }

    /// Searches for a key only in the local (=first) field.
    /// Does not search in ancestor (=second)
    pub fn contains_local(&self, k: &K) -> bool {
        self.0.contains_key(k)
    }

    /// Searches for a key in both fields. If a key is not locally bound,
    /// then ancestor environments are searched.
    pub fn contains(&self, k: &K) -> bool {
        let mut tmp = self;
        loop {
            if tmp.0.contains_key(k) {
                break true;
            } else {
                if let Some(ref e) = tmp.1 {
                    tmp = e.as_ref()
                } else {
                    break false;
                }
            }
        }
    }

    /// Insert an entry into the environment.
    /// Returns `None` if successful.
    /// If a key already exists, either locally or inherited,
    /// nothing happens and the provided key and value are
    /// returned in a `Some` variant.
    pub fn define(
        &mut self,
        k: K,
        v: V,
    ) -> Option<(K, V)> {
        if self.contains(&k) {
            Some((k, v))
        } else {
            self.0.insert(k, v);
            None
        }
    }

    /// Insert an entry into the environment at the local level.
    /// This will overwrite any existing entry sharing the same key.
    /// If the entry existed, the old value is returned in an
    /// `Option::Some` variant. Otherwise, returns `None`.
    pub fn set(
        &mut self,
        k: K,
        v: V,
    ) -> Option<V> {
        self.0.insert(k, v)
    }

    /// Get a reference to the value stored for a certain key.
    pub fn get(&self, k: &K) -> Option<&V> {
        if let Some(v) = self.0.get(k) {
            Some(v)
        } else {
            if let Some(ref p) = self.1 {
                p.get(k)
            } else {
                None
            }
        }
    }

    /// Get a mutable reference to the value stored for a provided key.
    pub fn get_mut(
        &mut self,
        k: &K,
    ) -> Option<&mut V> {
        if let Some(v) = self.0.get_mut(k) {
            Some(v)
        } else {
            if let Some(ref mut p) = self.1 {
                p.get_mut(k)
            } else {
                None
            }
        }
    }

    /// Update the value of an entry matching a given key.
    /// If the key exists, the value is updated and a  
    /// reference to the newly inserted value is returned
    /// as an `Either::Left`.
    /// If the key doesn't exist, the provided value is returned
    /// as an `Either::Right`.
    pub fn update(
        &mut self,
        k: &K,
        v: V,
    ) -> Either<&V, V> {
        if let Some(v0) = self.get_mut(k) {
            *v0 = v;
            // safe to unwrap, since we know it exists
            Either::Left(self.get(k).unwrap())
        } else {
            Either::Right(v)
        }
    }

    /// Flattens the structure into a single environment with no
    /// parent. Shadowed bindings are overwritten in order of traversal,
    /// therefore newer bindings take precedence over older ones.
    pub fn flatten(self) -> Envr<K, V> {
        let mut env = Envr::new_from(None);
        let mut tmp = self;
        loop {
            for (k, v) in tmp.0 {
                env.0.insert(k, v);
            }
            if let Some(p) = tmp.1 {
                tmp = *p;
            } else {
                break;
            }
        }
        env
    }

    pub fn keylist(&self) -> Vec<Keys<K, V>> {
        let mut keylist = vec![];
        let mut tmp = self;
        loop {
            keylist.push(tmp.0.keys());
            if let Some(p) = &tmp.1 {
                tmp = p.as_ref();
            } else {
                break;
            }
        }
        keylist
    }

    pub fn keyset(&self) -> HashSet<&K> {
        let mut set = HashSet::new();
        let mut tmp = self;
        loop {
            for k in tmp.0.keys() {
                set.insert(k);
            }
            if let Some(p) = &tmp.1 {
                tmp = p.as_ref();
            } else {
                break;
            }
        }
        set
    }

    pub fn difference<'t>(
        &'t self,
        other: &'t Envr<K, V>,
    ) -> Envr<&'t K, &'t V> {
        let mut env = Envr::new_from(None);
        let this = self.keyset();
        let that = other.keyset();
        let keys = this.difference(&that);
        for k in keys {
            if let Some(v) = self.get(k) {
                env.set(*k, v);
            }
        }
        env
    }

    /// Return a map of keys and a vector containing all of their values
    ///  in order from newest to oldest, if shadowed
    pub fn stack(&self) -> HashMap<&K, Vec<&V>> {
        let mut map = HashMap::new();
        let mut tmp = self;
        loop {
            for (k, v) in &tmp.0 {
                map.entry(k)
                    .or_insert_with(
                        Vec::<&V>::new,
                    )
                    .push(v);
            }
            if let Some(p) = &tmp.1 {
                tmp = p.as_ref();
            } else {
                break;
            }
        }
        map.into_iter().collect()
    }
}

impl<K, V> PartialEq for Envr<K, V>
where
    K: Hashley,
    V: Hashley,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0) && self.1 == other.1
    }
}

impl<K, V, I> From<I> for Envr<K, V>
where
    I: IntoIterator<Item = (K, V)>,
    K: Hashley,
    V: Hashley,
{
    fn from(iter: I) -> Self {
        let mut env = Envr::new_from(None);
        for (k, v) in iter {
            env.0.insert(k, v);
        }
        env
    }
}

impl<K, V> std::fmt::Debug for Envr<K, V>
where
    K: std::fmt::Debug,
    V: std::fmt::Debug,
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match &self.1 {
            Some(parent) => f
                .debug_struct("Environment")
                .field("local", &(self.0))
                .field("parent", parent)
                .finish(),
            _ => f
                .debug_tuple("Environment")
                .field(&(self.0))
                .finish(),
        }
    }
}

impl<K, V> std::fmt::Display for Envr<K, V>
where
    K: std::fmt::Display,
    V: std::fmt::Display,
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str("{\n")?;
        for (k, v) in &self.0 {
            f.write_str("  ")?;
            K::fmt(k, f)?;
            f.write_str(" = ")?;
            V::fmt(v, f)?;
            f.write_str(",\n")?;
        }
        if let Some(p) = &self.1 {
            f.write_str("  parent = ")?;
            let parent = format!("{}", p);
            let parent = parent
                .split('\n')
                .map(|s| format!("  {}\n", s))
                .collect::<String>();
            f.write_fmt(format_args!(
                "{}",
                parent.trim()
            ))?;
            f.write_char('\n')?;
        }
        f.write_char('}')
    }
}

#[cfg(test)]
mod tests {
    use crate::Envr;

    #[test]
    fn it_works() {
        let env = Envr::from([
            (1, "a"),
            (2, "b"),
            (3, "c"),
        ]);
        println!("{}", &env);
        let mut env = env.extend();
        env.set(4, "d");
        println!("{}", &env);
        let mut env = env.extend();
        env.define(5, "e");
        env.set(4, "f");
        println!("{}", &env);
        let env = env.flatten();
        println!("{}", &env);
    }

    #[test]
    fn difference() {
        let env1 = Envr::from([
            ("a", 8),
            ("b", 9),
            ("c", 10),
        ]);
        let env2 = Envr::from([
            ("a", 9),
            ("d", 11),
            ("b", 4),
        ]);
        let diff = env1.difference(&env2);
        println!(
            "env1: {}\nenv2: {}\ndifference: {}",
            &env1, &env2, &diff
        );
        assert_eq!(
            diff,
            Envr::from([(&"c", &10)])
        )
    }
}
