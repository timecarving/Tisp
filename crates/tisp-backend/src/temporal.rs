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

    /// 惰性逐元素映射(§1 基于流编程):tail 保留原始流的惰性结构
    pub fn map<U>(self, f: Arc<dyn Fn(&T) -> U + Send + Sync>) -> Stream<U>
    where U: Clone + std::fmt::Debug + Send + 'static {
        let head = f(&self.head);
        let head2 = head.clone();
        let this = self.clone();
        let tail: Thunk<U> = Arc::new(Mutex::new(Some(Box::new(move || {
            match this.next() {
                Some(s) => s.map(f),
                None => Stream::repeat(head2.clone()),
            }
        }))));
        Stream { head, tail }
    }

    /// 惰性过滤:跳过不满足谓词的元素(§1 基于流编程)
    pub fn filter(self, p: Arc<dyn Fn(&T) -> bool + Send + Sync>) -> Stream<T> {
        if p(&self.head) {
            let this = self.clone();
            let tail: Thunk<T> = Arc::new(Mutex::new(Some(Box::new(move || {
                match this.next() {
                    Some(s) => s.filter(p),
                    None => Stream::repeat(this.head.clone()),
                }
            }))));
            Stream { head: self.head, tail }
        } else if let Some(next) = self.next() {
            next.filter(p)
        } else {
            Stream::repeat(self.head)
        }
    }

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

/// §18 空间回收:⃝(next)值在两时刻(两次 advance)后回收,无空间泄漏。
/// `delay v` 创建 next 值;`advance` 推进一个时刻;第二次 advance 后值被回收
/// (内存清空,后续访问返回 None)。
#[derive(Debug, Clone)]
pub struct Next<T: Clone> {
    value: Option<T>,
    ticks: u64,
}

impl<T: Clone> Next<T> {
    pub fn delay(v: T) -> Self {
        Next { value: Some(v), ticks: 0 }
    }

    /// 推进一个时刻;两次 advance 后回收值(返回 None,无空间泄漏)
    pub fn advance(&mut self) -> Option<T> {
        if self.value.is_none() {
            return None; // 已回收
        }
        self.ticks += 1;
        if self.ticks >= 2 {
            // 两时刻后回收:取出并清空,归还内存
            self.value.take()
        } else {
            self.value.clone()
        }
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

    /// §18 空间回收:⃝ 值两次 advance 后回收(无泄漏)
    #[test]
    fn test_next_recycled_after_two_advances() {
        let mut n = Next::delay(42);
        // 第一次 advance:仍可用
        assert_eq!(n.advance(), Some(42));
        // 第二次 advance:回收值(取出并清空)
        assert_eq!(n.advance(), Some(42));
        // 第三次 advance:已回收,返回 None
        assert_eq!(n.advance(), None);
    }
}
