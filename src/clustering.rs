pub fn cluster_values(values: &[f64], tolerance: f64) -> Vec<Vec<f64>> {
    if values.is_empty() {
        return Vec::new();
    }
    let mut xs = values.to_vec();
    xs.sort_by(|a, b| a.total_cmp(b));

    if tolerance == 0.0 || xs.len() == 1 {
        return xs.into_iter().map(|x| vec![x]).collect();
    }

    let mut groups: Vec<Vec<f64>> = vec![vec![xs[0]]];
    let mut last = xs[0];
    for x in xs.into_iter().skip(1) {
        if x <= last + tolerance {
            groups.last_mut().expect("non-empty").push(x);
        } else {
            groups.push(vec![x]);
        }
        last = x;
    }
    groups
}

pub fn cluster_items<T, F>(items: &[T], key_fn: F, tolerance: f64) -> Vec<Vec<T>>
where
    T: Clone,
    F: Fn(&T) -> f64,
{
    if items.is_empty() {
        return Vec::new();
    }
    let mut xs: Vec<(f64, T)> = items.iter().cloned().map(|item| (key_fn(&item), item)).collect();
    xs.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut groups: Vec<Vec<T>> = vec![vec![xs[0].1.clone()]];
    let mut last = xs[0].0;

    for (key, item) in xs.into_iter().skip(1) {
        if key <= last + tolerance {
            groups.last_mut().expect("non-empty").push(item);
        } else {
            groups.push(vec![item]);
        }
        last = key;
    }
    groups
}
