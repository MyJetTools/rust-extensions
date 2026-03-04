use std::slice::Iter;

pub struct VecMaybeStackIterator<'a, T> {
    stack_iterator: Option<Iter<'a, T>>,
    heap_iterator: Option<Iter<'a, T>>,
}

impl<'a, T> VecMaybeStackIterator<'a, T> {
    pub fn new(stack: Iter<'a, T>, heap: Iter<'a, T>) -> Self {
        Self {
            stack_iterator: Some(stack),
            heap_iterator: Some(heap),
        }
    }
}

impl<'a, T> Iterator for VecMaybeStackIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = &mut self.stack_iterator {
            let result = iter.next();

            if result.is_some() {
                return result;
            } else {
                self.stack_iterator = None;
            }
        }

        if let Some(iter) = &mut self.heap_iterator {
            let result = iter.next();

            if result.is_some() {
                return result;
            } else {
                self.heap_iterator = None;
            }
        }

        None
    }
}
