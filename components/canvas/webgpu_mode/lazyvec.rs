/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::ops;

pub type Epoch = u32;

pub struct LazyVec<T> {
    inner: Vec<(Epoch, Option<T>)>,
}

impl<T> LazyVec<T> {
    pub fn new() -> Self {
        LazyVec {
            inner: Vec::new(),
        }
    }

    pub fn push(&mut self, value: T) -> (Epoch, usize) {
        let id = self.inner.len(); //TODO: recycle
        let epoch = 1;
        self.inner.push((epoch, Some(value)));
        (epoch, id)
    }

    pub fn pop(&mut self, index: usize) -> Option<T> {
        self.inner[index].1.take()
    }

    pub fn retain<F: Fn(&T) -> bool>(&mut self, fun: F) {
        for &mut (_, ref mut option) in &mut self.inner {
            let keep = match *option {
                Some(ref value) => fun(value),
                None => true,
            };
            if !keep {
                *option = None;
            }
        }
    }
}

impl<T> ops::Index<usize> for LazyVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        self.inner[index].1.as_ref().unwrap()
    }
}
impl<T> ops::IndexMut<usize> for LazyVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.inner[index].1.as_mut().unwrap()
    }
}
