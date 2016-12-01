/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::WebMetalDeviceRequest;
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::WebMetalRenderPipelineStateBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use ipc_channel::ipc::{self, IpcSender};
use std::collections::hash_map::{Entry, HashMap};
use webmetal;

#[dom_struct]
pub struct WebMetalRenderPipelineState {
    reflector: Reflector,
    #[ignore_heap_size_of = "Defined in webmetal"]
    desc: webmetal::PipelineDesc,
    #[ignore_heap_size_of = "Defined in ipc-channel"]
    ipc_device: IpcSender<WebMetalDeviceRequest>,
    #[ignore_heap_size_of = "Defined in webmetal"]
    pipelines: DOMRefCell<HashMap<webmetal::RenderPassKey, webmetal::Pipeline>>,
}

impl WebMetalRenderPipelineState {
    pub fn new(global: &GlobalScope,
               desc: webmetal::PipelineDesc,
               ipc_device: IpcSender<WebMetalDeviceRequest>)
               -> Root<WebMetalRenderPipelineState> {
        let object = box WebMetalRenderPipelineState {
            reflector: Reflector::new(),
            desc: desc,
            ipc_device: ipc_device,
            pipelines: DOMRefCell::new(HashMap::new()),
        };
        reflect_dom_object(object, global, binding::Wrap)
    }

    pub fn get(&self, pass: &webmetal::RenderPass) -> webmetal::Pipeline {
        match self.pipelines.borrow_mut().entry(pass.get_inner()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let (sender, receiver) = ipc::channel().unwrap();
                let req = WebMetalDeviceRequest::MakeRenderPipeline(sender, self.desc.clone(), pass.clone());
                self.ipc_device.send(req).unwrap();
                let inner = receiver.recv().unwrap().unwrap();
                entry.insert(inner).clone()
            },
        }
    }
}

//impl binding::WebMetalRenderPipelineStateMethods for WebMetalRenderPipelineState {
//}
