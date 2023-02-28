use std::collections::BinaryHeap;

struct MergedIterator<T: Ord, I: Iterator<Item = T>> {
    items: BinaryHeap<IterBuf<T, I>>,
}

impl<T: Ord, I: Iterator<Item = T>> MergedIterator<T, I> {
    fn new() -> Self {
        MergedIterator {
            items: BinaryHeap::new(),
        }
    }

    fn add(&mut self, mut new: I) {
        self.items.push(IterBuf {
            buf: new.next(),
            iter: new,
        })
    }
}

impl<T: Ord, I: Iterator<Item = T>> Iterator for MergedIterator<T, I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.items.pop() {
            Some(mut next) => {
                if let Some(val) = next.buf {
                    // Advance the selected iterator and put it back in the heap if it has anything
                    // left.
                    next.buf = next.iter.next();
                    if next.buf.is_some() {
                        self.items.push(next);
                    }

                    return Some(val);
                }

                None // All iterators are empty
            }
            None => None, // Empty heap
        }
    }
}

struct IterBuf<T: Ord, I: Iterator<Item = T>> {
    iter: I,
    buf: Option<T>,
}

// Making a min heap
impl<T: Ord, I: Iterator<Item = T>> Ord for IterBuf<T, I> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (&self.buf, &other.buf) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(this_one), Some(other_one)) => other_one.cmp(&this_one),
        }
    }
}

impl<T: Ord, I: Iterator<Item = T>> PartialOrd for IterBuf<T, I> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord, I: Iterator<Item = T>> Eq for IterBuf<T, I> {}

impl<T: Ord, I: Iterator<Item = T>> PartialEq for IterBuf<T, I> {
    fn eq(&self, other: &Self) -> bool {
        self.buf == other.buf
    }
}

fn main() -> anyhow::Result<()> {
    let first = (1..5).into_iter();
    let second = (1..10).into_iter();
    let third = (8..13).into_iter();

    let mut merged = MergedIterator::new();

    for item in [first, second, third] {
        merged.add(item)
    }

    while let Some(next) = merged.next() {
        println!("{}", next);
    }

    Ok(())
}
