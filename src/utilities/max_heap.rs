use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// # Panics
/// All functions taking indices panic when given an out of bounds index
pub struct MaxHeap<T: Ord> {
    items: Vec<HeapItem<T>>,
}

struct HeapItem<T: Ord> {
    index: Arc<AtomicUsize>,
    item: T,
}

impl<T: Ord> MaxHeap<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
        }
    }

    pub fn extract_min(&mut self) -> Option<(Arc<AtomicUsize>, T)> {
        let item_count = self.len();

        // if no items return None
        if item_count == 0 {
            return None;
        }

        // if only one item, pop it
        if item_count == 1 {
            let last_item = self.items.pop().unwrap();
            return Some((last_item.index, last_item.item));
        }

        // swap root with last item
        self.items.swap(0, item_count - 1);
        let min_item = self.items.pop().unwrap();
        self.down_heap(0);

        Some((min_item.index, min_item.item))
    }

    pub fn peek(&self) -> Option<&T> {
        self.items.get(0).map(|item| &item.item)
    }

    pub fn get_ref(&self, index: usize) -> &T {
        &self.items[index].item
    }

    pub fn insert(&mut self, item: T) -> Arc<AtomicUsize> {
        let new_ref = Arc::new(AtomicUsize::new(0));
        self.insert_with_ref(item, new_ref.clone());
        new_ref
    }
    pub fn insert_with_ref(&mut self, item: T, ref_index: Arc<AtomicUsize>) {
        let new_heap_item = HeapItem {
            index: ref_index,
            item,
        };

        // place new item at bottom then bubble up
        self.items.push(new_heap_item);
        self.up_heap(self.items.len() - 1);
    }

    pub fn remove(&mut self, index: usize) -> (Arc<AtomicUsize>, T) {
        // swap item with last item and remove it
        let last_index = self.items.len() - 1;

        if last_index == index {
            // if item is last item just pop and return
            let item = self.items.pop().unwrap();
            return (item.index, item.item);
        }

        self.items.swap(index, last_index);
        let item = self.items.pop().unwrap();

        // up_heap or down_heap as nescessary
        if item.item > self.items[index].item {
            // swapped item is smaller, try down_heap
            self.down_heap(index);
        } else {
            // swapped item is larger, try up_heap
            self.up_heap(index);
        }

        (item.index, item.item)
    }

    /// modification closure should return true if ordering of item increased (or stays the same) and false if it decreased
    pub fn modify_key<F>(&mut self, index: usize, modification: F)
    where
        F: FnOnce(&mut T) -> bool,
    {
        if modification(&mut self.items[index].item) {
            self.up_heap(index);
        } else {
            self.down_heap(index);
        }
    }

    /// bubble up item so long as it is larger than parent
    /// assumes item at `index` does not have index set
    /// # Panics
    /// Panics if index >= total no. of items
    pub fn up_heap(&mut self, index: usize) {
        // early exit if already at top
        if index == 0 {
            self.items[index].index.store(index, Ordering::Release);
            return;
        }

        let parent_index = Self::parent_index(index);

        let item = &self.items[index];
        let parent = &self.items[parent_index];

        if item.item > parent.item {
            // swap with parent
            self.items.swap(index, parent_index);
            self.items[index].index.store(index, Ordering::Release);
            self.up_heap(parent_index);
        } else {
            // item in final position
            item.index.store(index, Ordering::Release);
        }
    }

    /// bubble down item so long as it is smaller than one child
    /// assumes item at `index` does not have index set
    /// # Panics
    /// Panics if index >= total no. of items
    pub fn down_heap(&mut self, index: usize) {
        let left_child_index = Self::left_child_index(index);
        let item = &self.items[index];

        if left_child_index >= self.items.len() {
            // no child
            item.index.store(index, Ordering::Release);
        } else if left_child_index == self.items.len() - 1 {
            // one child
            if item.item < self.items[left_child_index].item {
                // swap with left child
                self.items.swap(index, left_child_index);
                self.items[left_child_index]
                    .index
                    .store(left_child_index, Ordering::Release);
            }
            // since child is last item, this is the end of the road
            self.items[index].index.store(index, Ordering::Release);
        } else {
            // two children
            let larger_child_index =
                if self.items[left_child_index].item > self.items[left_child_index + 1].item {
                    left_child_index
                } else {
                    left_child_index + 1
                };

            if item.item < self.items[larger_child_index].item {
                // swap with larger child
                self.items.swap(index, larger_child_index);
                self.items[index].index.store(index, Ordering::Release);
                self.down_heap(larger_child_index);
            } else {
                // item in final position
                item.index.store(index, Ordering::Release);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    fn left_child_index(index: usize) -> usize {
        index * 2 + 1
    }

    /// # Panics
    /// Panics if given an index of 0
    ///
    /// ```
    /// assert_eq!((2usize - 1) / 2, 0);
    /// ```
    fn parent_index(index: usize) -> usize {
        (index - 1) / 2
    }
}
