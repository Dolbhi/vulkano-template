use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    usize,
};

/// # Panics
/// All functions taking indices panic when given an out of bounds index
#[derive(Default)]
pub struct MaxHeap<K: Ord + Copy, T> {
    items: Vec<HeapItem<K, T>>,
}

struct HeapItem<K: Ord + Copy, T> {
    index: Arc<AtomicUsize>,
    key: K,
    item: T,
}

pub struct HeapIter<'a, K: Ord + Copy, T> {
    next_index: usize,
    heap: &'a MaxHeap<K, T>,
}

impl<K: Ord + Copy, T> MaxHeap<K, T> {
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

        min_item.index.store(usize::MAX, Ordering::Release);
        Some((min_item.index, min_item.item))
    }

    pub fn peek(&self) -> Option<&T> {
        self.items.first().map(|item| &item.item)
    }

    pub fn get_ref(&self, index: usize) -> &T {
        &self.items[index].item
    }

    pub fn insert(&mut self, key: K, item: T) -> Arc<AtomicUsize> {
        let new_ref = Arc::new(AtomicUsize::new(0));
        self.insert_with_ref(key, item, new_ref.clone());
        new_ref
    }
    pub fn insert_with_ref(&mut self, key: K, item: T, ref_index: Arc<AtomicUsize>) {
        let new_heap_item = HeapItem {
            index: ref_index,
            key,
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
        if item.key > self.items[index].key {
            // swapped item is smaller, try down_heap
            self.down_heap(index);
        } else {
            // swapped item is larger, try up_heap
            self.up_heap(index);
        }

        item.index.store(usize::MAX, Ordering::Release);
        (item.index, item.item)
    }

    /// modification closure should calculate and return the new key
    pub fn modify_key<F>(&mut self, index: usize, modification: F)
    where
        F: FnOnce(&mut T) -> K,
    {
        let item = &mut self.items[index];
        let new_key = modification(&mut item.item);

        if new_key > item.key {
            item.key = new_key;
            self.up_heap(index);
        } else {
            item.key = new_key;
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

        if item.key > parent.key {
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
            if item.key < self.items[left_child_index].key {
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
                if self.items[left_child_index].key > self.items[left_child_index + 1].key {
                    left_child_index
                } else {
                    left_child_index + 1
                };

            if item.key < self.items[larger_child_index].key {
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
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn left_child_index(index: usize) -> usize {
        index * 2 + 1
    }

    pub fn iter(&self) -> HeapIter<K, T> {
        HeapIter {
            next_index: 0,
            heap: self,
        }
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

impl<K: Ord + Copy, T> Iterator for MaxHeap<K, T> {
    type Item = (Arc<AtomicUsize>, T);
    fn next(&mut self) -> Option<Self::Item> {
        self.extract_min()
    }
}

impl<'a, K: Ord + Copy, T> Iterator for HeapIter<'a, K, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.heap.items.get(self.next_index);
        self.next_index += 1;
        next.map(|heap_item| &heap_item.item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.heap.len() - self.next_index;
        (size, Some(size))
    }
}
impl<'a, K: Ord + Copy, T> ExactSizeIterator for HeapIter<'a, K, T> {}
