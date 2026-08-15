/// Region-based memory allocator — Tofte-Talpin style
/// Provides stack-of-regions memory management without GC



/// A region identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionId(pub u64);

/// Region allocation/disposal
#[derive(Debug)]
enum RegionKind {
    /// Finite region — allocated on the C stack
    Finite {
        ptr: *mut u8,
        size: usize,
        used: usize,
    },
    /// Infinite region — linked list of fixed-size pages
    Infinite {
        pages: Vec<Vec<u8>>,
        current_page: usize,
        page_offset: usize,
        page_size: usize,
    },
    /// Scalar region — no allocation (value fits in word/register)
    Scalar,
}

/// A region in the region stack
#[derive(Debug)]
struct Region {
    kind: RegionKind,
    id: RegionId,
}

/// The region stack manager
pub struct RegionStack {
    stack: Vec<Region>,
    next_id: u64,
    page_size: usize,
    /// 区域内分配对象的析构钩子:(region id, drop fn);pop 时先析构再释放内存
    drop_hooks: Vec<(RegionId, Box<dyn FnOnce()>)>,
    pub stats: RegionStats,
}

#[derive(Debug, Default, Clone)]
pub struct RegionStats {
    pub regions_allocated: u64,
    pub regions_deallocated: u64,
    pub bytes_allocated: u64,
    pub bytes_peak: u64,
}

impl RegionStack {
    pub fn new(page_size: usize) -> Self {
        Self {
            stack: Vec::new(),
            next_id: 0,
            page_size: page_size.max(4096),
            drop_hooks: Vec::new(),
            stats: RegionStats::default(),
        }
    }

    /// Push a new finite region (size known at compile time)
    pub fn push_finite_region(&mut self, size: usize) -> RegionId {
        let id = self.fresh_id();
        let layout = std::alloc::Layout::from_size_align(size, 16).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        self.stats.regions_allocated += 1;
        self.stats.bytes_allocated += size as u64;
        if self.stats.bytes_allocated > self.stats.bytes_peak {
            self.stats.bytes_peak = self.stats.bytes_allocated;
        }
        self.stack.push(Region {
            kind: RegionKind::Finite { ptr, size, used: 0 },
            id,
        });
        id
    }

    /// Push a new infinite region (size not known, grows as needed)
    pub fn push_infinite_region(&mut self) -> RegionId {
        let id = self.fresh_id();
        let page_size = self.page_size;
        let first_page = vec![0u8; page_size];
        self.stats.regions_allocated += 1;
        self.stats.bytes_allocated += page_size as u64;
        self.stack.push(Region {
            kind: RegionKind::Infinite {
                pages: vec![first_page],
                current_page: 0,
                page_offset: 0,
                page_size,
            },
            id,
        });
        id
    }

    /// Push a scalar region (no actual allocation)
    pub fn push_scalar_region(&mut self) -> RegionId {
        let id = self.fresh_id();
        self.stack.push(Region { kind: RegionKind::Scalar, id });
        id
    }

    /// Pop the top region, freeing all its memory
    pub fn pop_region(&mut self) {
        if let Some(region) = self.stack.pop() {
            self.run_drop_hooks(region.id);
            self.stats.regions_deallocated += 1;
            match region.kind {
                RegionKind::Finite { ptr, size, .. } => {
                    self.stats.bytes_allocated -= size as u64;
                    unsafe {
                        std::alloc::dealloc(ptr, std::alloc::Layout::from_size_align(size, 16).unwrap());
                    }
                }
                RegionKind::Infinite { pages, page_size, .. } => {
                    self.stats.bytes_allocated -= (pages.len() * page_size) as u64;
                    // Pages are Vec<u8>, dropped automatically
                }
                RegionKind::Scalar => {}
            }
        }
    }

    /// Reset a region (clear contents, reuse memory)
    pub fn reset_region(&mut self, id: RegionId) {
        for region in self.stack.iter_mut() {
            if region.id == id {
                match &mut region.kind {
                    RegionKind::Finite { used, .. } => *used = 0,
                    RegionKind::Infinite { pages: _, current_page, page_offset, .. } => {
                        *current_page = 0;
                        *page_offset = 0;
                        // Keep pages allocated
                    }
                    RegionKind::Scalar => {}
                }
                return;
            }
        }
    }

    /// Allocate within a region. Returns pointer to allocated memory.
    pub fn region_alloc(&mut self, id: RegionId, size: usize) -> Option<*mut u8> {
        for region in self.stack.iter_mut().rev() {
            if region.id == id {
                return match &mut region.kind {
                    RegionKind::Finite { ptr, size: region_size, used } => {
                        let align = 16;
                        let offset = (*used + align - 1) & !(align - 1);
                        if offset + size <= *region_size {
                            *used = offset + size;
                            Some(unsafe { ptr.add(offset) })
                        } else {
                            None // Region exhausted
                        }
                    }
                    RegionKind::Infinite { pages, current_page, page_offset, page_size } => {
                        let align = 16;
                        let offset = (*page_offset + align - 1) & !(align - 1);
                        if offset + size > *page_size {
                            // Need new page
                            *current_page += 1;
                            if *current_page >= pages.len() {
                                pages.push(vec![0u8; *page_size]);
                                self.stats.bytes_allocated += *page_size as u64;
                            }
                            *page_offset = size;
                            Some(pages[*current_page].as_mut_ptr())
                        } else {
                            *page_offset = offset + size;
                            Some(unsafe { pages[*current_page].as_mut_ptr().add(offset) })
                        }
                    }
                    RegionKind::Scalar => None,
                };
            }
        }
        None
    }

    /// Current depth of the region stack
    pub fn depth(&self) -> usize { self.stack.len() }

    /// Check if a region is still alive
    pub fn is_alive(&self, id: RegionId) -> bool {
        self.stack.iter().any(|r| r.id == id)
    }

    /// 注册区域对象的析构钩子(区域 pop 时按注册序调用,在内存释放前)
    pub fn register_drop(&mut self, id: RegionId, hook: Box<dyn FnOnce()>) {
        self.drop_hooks.push((id, hook));
    }

    fn run_drop_hooks(&mut self, id: RegionId) {
        let mut i = 0;
        while i < self.drop_hooks.len() {
            if self.drop_hooks[i].0 == id {
                let (_, hook) = self.drop_hooks.remove(i);
                hook();
            } else {
                i += 1;
            }
        }
    }

    fn fresh_id(&mut self) -> RegionId {
        let id = RegionId(self.next_id);
        self.next_id += 1;
        id
    }
}

impl Drop for RegionStack {
    fn drop(&mut self) {
        // Pop all regions to free memory
        while !self.stack.is_empty() {
            self.pop_region();
        }
    }
}

/// 区域盒(§统一内存管理):值真实分配在 RegionStack 区域内,
/// 区域 pop 时经析构钩子调用 drop(T) 后再释放底层内存。
pub struct RegionBox<T> {
    ptr: *mut T,
    region: RegionId,
    _marker: std::marker::PhantomData<T>,
}

impl<T: 'static> RegionBox<T> {
    /// 在指定区域分配并写入 value;分配失败(区域耗尽/已死)返回 None
    pub fn new_in(stack: &mut RegionStack, region: RegionId, value: T) -> Option<Self> {
        let bytes = std::mem::size_of::<T>();
        let ptr = stack.region_alloc(region, bytes.max(1))? as *mut T;
        unsafe { ptr.write(value); }
        let hook_ptr = ptr;
        stack.register_drop(region, Box::new(move || unsafe {
            std::ptr::drop_in_place(hook_ptr);
        }));
        Some(Self { ptr, region, _marker: std::marker::PhantomData })
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.ptr }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }

    pub fn region(&self) -> RegionId {
        self.region
    }
}

// RegionBox 本身不释放:值随区域生命周期结束由 RegionStack 的析构钩子回收
impl<T> Drop for RegionBox<T> {
    fn drop(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_box_drop_hook() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static DROPS: AtomicUsize = AtomicUsize::new(0);
        struct CountDrop;
        impl Drop for CountDrop {
            fn drop(&mut self) { DROPS.fetch_add(1, Ordering::SeqCst); }
        }
        let mut rs = RegionStack::new(4096);
        let id = rs.push_finite_region(1024);
        {
            let _boxed = RegionBox::new_in(&mut rs, id, CountDrop).expect("区域应可分配");
            assert_eq!(DROPS.load(Ordering::SeqCst), 0);
        }
        // RegionBox drop 不析构;区域 pop 时执行钩子
        assert_eq!(DROPS.load(Ordering::SeqCst), 0);
        rs.pop_region();
        assert_eq!(DROPS.load(Ordering::SeqCst), 1, "区域回收应先析构对象");
    }

    #[test]
    fn test_finite_region() {
        let mut rs = RegionStack::new(4096);
        let id = rs.push_finite_region(1024);
        assert!(rs.is_alive(id));

        let ptr = rs.region_alloc(id, 64);
        assert!(ptr.is_some());

        rs.pop_region();
        assert!(!rs.is_alive(id));
    }

    #[test]
    fn test_infinite_region() {
        let mut rs = RegionStack::new(256);
        let id = rs.push_infinite_region();

        // Allocate beyond one page
        for _ in 0..10 {
            assert!(rs.region_alloc(id, 100).is_some());
        }
        assert!(rs.is_alive(id));
        rs.pop_region();
    }

    #[test]
    fn test_region_reset() {
        let mut rs = RegionStack::new(4096);
        let id = rs.push_finite_region(1024);

        let ptr1 = rs.region_alloc(id, 64);
        rs.reset_region(id);
        let ptr2 = rs.region_alloc(id, 64);

        // After reset, should allocate at the same position
        assert_eq!(ptr1, ptr2);
        rs.pop_region();
    }

    #[test]
    fn test_scalar_region() {
        let mut rs = RegionStack::new(4096);
        let id = rs.push_scalar_region();
        assert!(rs.region_alloc(id, 8).is_none()); // No allocation in scalar
        rs.pop_region();
        assert_eq!(rs.stats.bytes_allocated, 0);
    }
}
