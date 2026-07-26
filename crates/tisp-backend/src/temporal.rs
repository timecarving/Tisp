use std::sync::{Arc, Mutex};

type Thunk<T> = Arc<Mutex<Option<Box<dyn FnOnce() -> Stream<T> + Send>>>>;

#[derive(Clone)]
pub struct Stream<T: Clone + std::fmt::Debug> {
    head: T,
    tail: Thunk<T>,
}

impl<T: Clone + std::fmt::Debug + Send + 'static> Stream<T> {
    pub fn unfold(initial: T, step: fn(&T) -> T) -> Self {
        let next = step(&initial);
        let step: Thunk<T> = Arc::new(Mutex::new(Some(Box::new(move || Stream::unfold(next, step)))));
        Stream { head: initial, tail: step }
    }

    pub fn repeat(value: T) -> Self {
        let v = value.clone();
        let step: Thunk<T> = Arc::new(Mutex::new(Some(Box::new(move || Stream::repeat(v)))));
        Stream { head: value, tail: step }
    }

    pub fn now(&self) -> &T { &self.head }

    pub fn next(&self) -> Option<Stream<T>> {
        let mut guard = self.tail.lock().unwrap();
        if let Some(thunk) = guard.take() {
            Some(thunk())
        } else {
            None
        }
    }

    pub fn take(self, n: usize) -> Vec<T> {
        let mut result = vec![self.head.clone()];
        let mut current = self;
        for _ in 1..n {
            match current.next() {
                Some(s) => { result.push(s.head.clone()); current = s; }
                None => break,
            }
        }
        result
    }

    pub fn fold<U>(self, n: usize, init: U, f: fn(U, &T) -> U) -> U {
        let mut acc = f(init, &self.head);
        let mut current = self;
        for _ in 1..n {
            match current.next() {
                Some(s) => { acc = f(acc, &s.head); current = s; }
                None => break,
            }
        }
        acc
    }
}

impl<T: Clone + std::fmt::Debug> std::fmt::Debug for Stream<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stream({:?}, ...)", self.head)
    }
}

#[derive(Debug, Clone)]
pub struct Clock {
    pub name: String,
    pub tick_rate_hz: f64,
    pub current_tick: u64,
}

impl Clock {
    pub fn new(name: &str, hz: f64) -> Self { Self { name: name.to_string(), tick_rate_hz: hz, current_tick: 0 } }
    pub fn tick(&mut self) -> u64 { self.current_tick += 1; self.current_tick }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_stream_take() {
        let s = Stream::unfold(1, |n| n + 1);
        assert_eq!(s.take(5), vec![1, 2, 3, 4, 5]);
    }
    #[test]
    fn test_repeat() { assert_eq!(Stream::repeat(42).take(3), vec![42, 42, 42]); }
    #[test]
    fn test_fold() {
        let s = Stream::unfold(1, |n| n + 1);
        assert_eq!(s.fold(5, 0, |a, n| a + n), 15);
    }
}
