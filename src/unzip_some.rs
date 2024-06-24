use rayon::prelude::*;
use std::convert::identity;

struct OnlySome<T>(Vec<T>);

impl<T> Default for OnlySome<T> {
    fn default() -> Self {
        OnlySome(Vec::new())
    }
}

impl<T: Send> ParallelExtend<Option<T>> for OnlySome<T> {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = Option<T>>,
    {
        let some = par_iter.into_par_iter().filter_map(identity);
        self.0.par_extend(some);
    }
}

pub fn unzip_some<
    I,
    K: Send + Ord + std::hash::Hash,
    V: Send,
    U: Send,
    S: std::hash::BuildHasher + Default,
>(
    par_iter: I,
) -> (std::collections::HashMap<K, V, S>, Vec<U>)
where
    // let's assume we've already mapped to options
    I: ParallelIterator<
        Item = (
            Option<std::collections::hash_map::IntoIter<K, V>>,
            Option<U>,
        ),
    >,
{
    let (some_ts, some_us): (
        OnlySome<std::collections::hash_map::IntoIter<K, V>>,
        OnlySome<U>,
    ) = par_iter.unzip();
    (some_ts.0.into_iter().flatten().collect(), some_us.0)
}
