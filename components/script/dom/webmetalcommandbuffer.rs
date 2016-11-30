/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{CanvasMsg, WebMetalCommand};
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::WebMetalCommandBufferBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflectable, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webmetalrendercommandencoder::WebMetalRenderCommandEncoder;
use ipc_channel::ipc::{self, IpcSender};
use std::cell::Cell;
use std::rc::Rc;
use webmetal;
use webmetal_resource_proxy::WebMetalResourceProxy;

#[dom_struct]
pub struct WebMetalCommandBuffer {
    reflector: Reflector,
    #[ignore_heap_size_of = "Defined in ipc-channel"]
    ipc_renderer: IpcSender<CanvasMsg>,
    #[ignore_heap_size_of = "nothing to see here"]
    resource_proxy: Rc<DOMRefCell<WebMetalResourceProxy>>,
    #[ignore_heap_size_of = "Defined in webmetal"]
    inner: webmetal::CommandBuffer,
    sealed: Cell<bool>,
}

impl WebMetalCommandBuffer {
    pub fn new(global: &GlobalScope,
               ipc_renderer: IpcSender<CanvasMsg>,
               resource_proxy: Rc<DOMRefCell<WebMetalResourceProxy>>,
               inner: webmetal::CommandBuffer)
               -> Root<WebMetalCommandBuffer> {
        let object = box WebMetalCommandBuffer {
            reflector: Reflector::new(),
            ipc_renderer: ipc_renderer,
            resource_proxy: resource_proxy,
            inner: inner,
            sealed: Cell::new(false),
        };
        reflect_dom_object(object, global, binding::Wrap)
    }
}

impl binding::WebMetalCommandBufferMethods for WebMetalCommandBuffer {
    fn MakeRenderCommandEncoder(&self, targets: &binding::RenderTargetSet)
                                -> Root<WebMetalRenderCommandEncoder> {
        assert!(!self.sealed.get());
        //Note: this code is rough around the edge cases
        let colors = [&targets.color0, &targets.color1, &targets.color2, &targets.color3];
        let targetset = webmetal::TargetSet {
            colors: colors.into_iter()
                          .filter_map(|color| color.view.as_ref().map(|view| {
                              (view.get_inner(),
                               color.clear.as_ref().map(|v| [*v[0], *v[1], *v[2], *v[3]]))
                          })).collect(),
            depth_stencil: targets.depthStencil.view.as_ref().map(|view| {
                (view.get_inner(),
                 targets.depthStencil.clear.as_ref().and_then(|v| v.get(0).map(|&d| *d)),
                 targets.depthStencil.clear.as_ref().and_then(|v| v.get(1).map(|&s| *s as u8)))
            }),
        };
        let (sender, receiver) = ipc::channel().unwrap();
        let (render_pass, framebuf, clear_data) = self.resource_proxy.borrow_mut().make_render_pass(targetset);
        let msg = WebMetalCommand::StartRenderEncoder(receiver, self.inner.clone(), render_pass.clone(), framebuf, clear_data);
        self.ipc_renderer.send(CanvasMsg::WebMetal(msg)).unwrap();
        WebMetalRenderCommandEncoder::new(&self.global(), render_pass, sender)
    }

    fn Commit(&self) {
        self.sealed.set(true);
        let msg = WebMetalCommand::Submit(self.inner.clone());
        self.ipc_renderer.send(CanvasMsg::WebMetal(msg)).unwrap();
    }
}

impl Drop for WebMetalCommandBuffer {
    fn drop(&mut self) {
        //TODO
    }
}
