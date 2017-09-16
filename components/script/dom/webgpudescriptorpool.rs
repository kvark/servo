/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{webgpu_channel,
    DescriptorPoolId, DescriptorPoolInfo, WebGpuChan, WebGpuMsg,
};
use dom::bindings::codegen::Bindings::WebGpuDescriptorPoolBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::webgpudescriptorset::WebGpuDescriptorSet;
use dom::webgpudescriptorsetlayout::WebGpuDescriptorSetLayout;
use dom::globalscope::GlobalScope;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuDescriptorPool {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    id: DescriptorPoolId,
}

impl WebGpuDescriptorPool {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuChan,
        info: DescriptorPoolInfo,
    ) -> Root<Self> {
        let obj = box WebGpuDescriptorPool {
            reflector_: Reflector::new(),
            sender,
            id: info.id,
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }

    pub fn _get_id(&self) -> DescriptorPoolId {
        self.id
    }
}

impl binding::WebGpuDescriptorPoolMethods for WebGpuDescriptorPool {
    fn AllocateSets(
        &self,
        layouts: Vec<Root<WebGpuDescriptorSetLayout>>,
    ) -> Vec<Root<WebGpuDescriptorSet>> {
        let count = layouts.len();
        let (sender, receiver) = webgpu_channel().unwrap();

        let msg = WebGpuMsg::AllocateDescriptorSets {
            pool_id: self.id,
            set_layout_ids: layouts
                .into_iter()
                .map(|dsl| dsl.get_id())
                .collect(),
            result: sender,
        };
        self.sender.send(msg).unwrap();

        (0 .. count)
            .map(|_| {
                let set = receiver.recv().unwrap();
                WebGpuDescriptorSet::new(&self.global(), set)
            })
            .collect()
    }
}
