pub struct Zipper<T> {
    pub focus: T,
    pub up: Option<Box<dyn Fn(T) -> Zipper<T>>>,
}

pub fn zip<T>(focus: T) -> Zipper<T> {
    Zipper { focus, up: None }
}
