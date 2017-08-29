/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::ops;
use canvas_traits::webgpu::{Epoch, Key};

pub struct LazyVec<T> {
    inner: Vec<(Epoch, Option<T>)>,
}

impl<T> LazyVec<T> {
    pub fn new() -> Self {
        LazyVec {
            inner: Vec::new(),
        }
    }

    pub fn push(&mut self, value: T) -> Key {
        let key = Key {
            index: self.inner.len(), //TODO: recycle
            epoch: 1,
        };
        self.inner.push((key.epoch, Some(value)));
        key
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        let value = &mut self.inner[key.index];
        debug_assert_eq!(value.0, key.epoch);
        value.1.take()
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

impl<T> ops::Index<Key> for LazyVec<T> {
    type Output = T;
    fn index(&self, key: Key) -> &T {
        let value = &self.inner[key.index];
        debug_assert_eq!(value.0, key.epoch);
        value.1.as_ref().unwrap()
    }
}
impl<T> ops::IndexMut<Key> for LazyVec<T> {
    fn index_mut(&mut self, key: Key) -> &mut T {
        let value = &mut self.inner[key.index];
        debug_assert_eq!(value.0, key.epoch);
        value.1.as_mut().unwrap()
    }
}
