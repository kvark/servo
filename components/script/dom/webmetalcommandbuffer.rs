/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{CanvasMsg, WebMetalCommand};
use dom::bindings::codegen::Bindings::WebMetalCommandBufferBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflectable, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webmetalrenderencoder::WebMetalRenderEncoder;
use ipc_channel::ipc::{self, IpcSender};
use webmetal;

#[dom_struct]
pub struct WebMetalCommandBuffer {
    reflector: Reflector,
    #[ignore_heap_size_of = "Defined in ipc-channel"]
    ipc_renderer: IpcSender<CanvasMsg>,
    #[ignore_heap_size_of = "Defined in webmetal"]
    inner: webmetal::CommandBuffer,
}

impl WebMetalCommandBuffer {
    pub fn new(global: &GlobalScope,
               ipc_renderer: IpcSender<CanvasMsg>,
               inner: webmetal::CommandBuffer)
               -> Root<WebMetalCommandBuffer> {
        let object = box WebMetalCommandBuffer {
            reflector: Reflector::new(),
            ipc_renderer: ipc_renderer.clone(),
            inner: inner,
        };
        reflect_dom_object(object, global, binding::Wrap)
    }

    pub fn get_inner(&self) -> webmetal::CommandBuffer {
        self.inner.clone()
    }
}

impl binding::WebMetalCommandBufferMethods for WebMetalCommandBuffer {
    fn MakeRenderEncoder(&self, targets: &binding::RenderTargets)
                         -> Root<WebMetalRenderEncoder> {
        let (sender, receiver) = ipc::channel().unwrap();
        let colors = [&targets.color0, &targets.color1, &targets.color2, &targets.color3];
        let targetset = webmetal::TargetSet {
            colors: colors.into_iter()
                          .filter_map(|c| c.as_ref())
                          .map(|v| v.get_inner())
                          .collect(),
            depth: targets.depth.as_ref().map(|v| v.get_inner()),
            stencil: targets.stencil.as_ref().map(|v| v.get_inner()),
        };
        let msg = WebMetalCommand::MakeRenderEncoder(receiver, self.inner.clone(), targetset);
        self.ipc_renderer.send(CanvasMsg::WebMetal(msg)).unwrap();
        WebMetalRenderEncoder::new(&self.global(), sender)
    }
}

impl Drop for WebMetalCommandBuffer {
    fn drop(&mut self) {
        //TODO
    }
}
