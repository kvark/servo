/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WebMetalTargetViewBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use webmetal;

#[dom_struct]
pub struct WebMetalTargetView {
    reflector: Reflector,
    #[ignore_heap_size_of = "Defined in webmetal"]
    inner: webmetal::TargetView,
}

impl WebMetalTargetView {
    pub fn new(global: &GlobalScope, inner: webmetal::TargetView)
               -> Root<WebMetalTargetView> {
        let object = box WebMetalTargetView {
            reflector: Reflector::new(),
            inner: inner,
        };
        reflect_dom_object(object, global, binding::Wrap)
    }

    pub fn get_inner(&self) -> webmetal::TargetView {
        self.inner.clone()
    }
}
