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

#[cfg(test)]
mod tests {
    use super::*;

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
