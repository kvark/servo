/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//use canvas_traits::{CanvasCommonMsg, CanvasMsg, WebGpuCommand};
use canvas_traits::webgpu::{WebGpuMsgSender};
//use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::WebGpuRenderingContextBinding as binding;
use dom::bindings::js::{JS, LayoutJS, Root};
//use dom::bindings::inheritance::Castable;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::htmlcanvaselement::HTMLCanvasElement;
//use dom::node::{Node, NodeDamage};
//use dom::webmetalcommandbuffer::WebMetalCommandBuffer;
//use dom::webmetaldevice::WebMetalDevice;
//use dom::webmetaltargetview::WebMetalTargetView;
use dom_struct::dom_struct;
use euclid::Size2D;
use script_layout_interface::HTMLCanvasDataSource;
//use script_traits::ScriptMsg as ConstellationMsg;
//use std::cell::Cell;
//use std::rc::Rc;
//use webmetal_resource_proxy::WebMetalResourceProxy;
//use webmetal::WebMetalCapabilities;

#[dom_struct]
pub struct WebGpuRenderingContext {
    reflector: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    webgpu_sender: WebGpuMsgSender,
    //#[ignore_heap_size_of = "Defined in webmetal"]
    //capabilities: WebMetalCapabilities,
    canvas: JS<HTMLCanvasElement>,
    //device: JS<WebMetalDevice>,
    //#[ignore_heap_size_of = "nothing to see here"]
    //resource_proxy: Rc<DOMRefCell<WebMetalResourceProxy>>,
    //current_target_index: Cell<usize>,
    //swap_targets: Vec<JS<WebMetalTargetView>>,
}

impl WebGpuRenderingContext {
    fn new_internal(global: &GlobalScope, canvas: &HTMLCanvasElement, size: Size2D<i32>)
                    -> Result<WebGpuRenderingContext, String> {
        /*
        let (sender, receiver) = ipc::channel().unwrap();
        let num_frames = 3;
        global.constellation_chan()
              .send(ConstellationMsg::CreateWebMetalPaintThread(size, num_frames, sender))
              .unwrap();
        let response = receiver.recv().unwrap();
        response.map(|(ipc_context, ipc_device, targets, caps)| WebMetalRenderingContext {
            reflector: Reflector::new(),
            ipc_renderer: ipc_context,
            capabilities: caps,
            canvas: JS::from_ref(canvas),
            device: JS::from_ref(&*WebMetalDevice::new(global, ipc_device.clone())),
            resource_proxy: Rc::new(DOMRefCell::new(WebMetalResourceProxy::new(ipc_device))),
            current_target_index: Cell::new(0),
            swap_targets: targets.into_iter().map(|view|
                JS::from_ref(&*WebMetalTargetView::new(global, view))
                ).collect(),
        })*/
        unimplemented!()
    }

    #[allow(unrooted_must_root)]
    pub fn new(global: &GlobalScope, canvas: &HTMLCanvasElement, size: Size2D<i32>)
               -> Option<Root<WebGpuRenderingContext>> {
        match Self::new_internal(global, canvas, size) {
            Ok(ctx) => Some(reflect_dom_object(box ctx, global, binding::Wrap)),
            Err(msg) => {
                error!("Couldn't create WebGpuRenderingContext: {}", msg);
                //TODO: error event?
                None
            }
        }
    }

    pub fn recreate(&self, size: Size2D<i32>) {
        unimplemented!()
        //self.ipc_renderer.send(CanvasMsg::Common(CanvasCommonMsg::Recreate(size))).unwrap();
    }

    pub fn webgpu_sender(&self) -> WebGpuMsgSender {
        self.webgpu_sender.clone()
    }
}

impl binding::WebGpuRenderingContextMethods for WebGpuRenderingContext {
    /*fn GetDevice(&self) -> Root<WebMetalDevice> {
        Root::from_ref(&self.device)
    }

    fn MakeCommandBuffer(&self) -> Root<WebMetalCommandBuffer> {
        let com = self.resource_proxy.borrow_mut().make_command_buffer();
        WebMetalCommandBuffer::new(&self.global(),
                                   self.ipc_renderer.clone(),
                                   self.resource_proxy.clone(),
                                   com)
    }

    fn NextFrameTarget(&self) -> Root<WebMetalTargetView> {
        let mut index = self.current_target_index.get() + 1;
        if index >= self.swap_targets.len() {
            index = 0;
        }
        self.current_target_index.set(index);
        Root::from_ref(&self.swap_targets[index])
    }*/

    fn EndFrame(&self) {
        unimplemented!()
        //let msg = WebMetalCommand::Present(self.current_target_index.get() as u32);
        //self.ipc_renderer.send(CanvasMsg::WebMetal(msg)).unwrap();
        //TODO: wait for a fence
        //self.canvas.upcast::<Node>().dirty(NodeDamage::OtherNodeDamage);
    }
}

pub trait LayoutCanvasWebGpuRenderingContextHelpers {
    #[allow(unsafe_code)]
    unsafe fn canvas_data_source(&self) -> HTMLCanvasDataSource;
}

impl LayoutCanvasWebGpuRenderingContextHelpers for LayoutJS<WebGpuRenderingContext> {
    #[allow(unsafe_code)]
    unsafe fn canvas_data_source(&self) -> HTMLCanvasDataSource {
        unimplemented!()
        //HTMLCanvasDataSource::WebGpu((*self.unsafe_get()).layout_handle())
    }
}
