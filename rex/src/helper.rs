pub fn push_new<T: Clone>(mut v: Vec<T>, elem: T) -> Vec<T> {
    v.push(elem);
    v
}
