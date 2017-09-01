/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{WebGpuChan, WebGpuMsg, webgpu_channel};
use dom::bindings::codegen::Bindings::WebGpuRenderingContextBinding as binding;
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding::WebGpuFormat;
use dom::bindings::js::{JS, LayoutJS, Root};
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::htmlcanvaselement::HTMLCanvasElement;
use dom::webgpuadapter::WebGpuAdapter;
use dom::webgpudevice::WebGpuDevice;
use dom::webgpucommandqueue::WebGpuCommandQueue;
use dom::webgpuswapchain::WebGpuSwapchain;
use dom::window::Window;
use dom_struct::dom_struct;
use script_layout_interface::HTMLCanvasDataSource;


#[dom_struct]
pub struct WebGpuRenderingContext {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    canvas: JS<HTMLCanvasElement>,
    adapters: Vec<Root<WebGpuAdapter>>,
}

impl WebGpuRenderingContext {
    fn new_internal(
        window: &Window,
        canvas: &HTMLCanvasElement,
    ) -> Result<WebGpuRenderingContext, String>
    {
        let (sender, receiver) = webgpu_channel().unwrap();
        let webgpu_chan = window.webgpu_chan();

        webgpu_chan
            .send(WebGpuMsg::CreateContext(sender))
            .unwrap();
        receiver
            .recv()
            .unwrap()
            .map(|init| {
                let sender = init.sender.sender.clone();
                let adapters = init.adapters
                    .into_iter()
                    .map(|info| WebGpuAdapter::new(window, sender.clone(), info))
                    .collect();
                WebGpuRenderingContext {
                    reflector_: Reflector::new(),
                    sender,
                    adapters,
                    canvas: JS::from_ref(canvas),
                }
            })
    }

    #[allow(unrooted_must_root)]
    pub fn new(
        window: &Window,
        canvas: &HTMLCanvasElement,
    ) -> Option<Root<WebGpuRenderingContext>> {
        match Self::new_internal(window, canvas) {
            Ok(ctx) => Some(reflect_dom_object(box ctx, window, binding::Wrap)),
            Err(msg) => {
                error!("Couldn't create WebGpuRenderingContext: {}", msg);
                //TODO: error event?
                None
            }
        }
    }

    pub fn recreate(&self) {
        //TODO
    }
}

impl binding::WebGpuRenderingContextMethods for WebGpuRenderingContext {
    fn EnumerateAdapters(&self) -> Vec<Root<WebGpuAdapter>> {
        self.adapters.clone()
    }
    fn BuildSwapchain(&self, queue: &WebGpuCommandQueue) -> Root<WebGpuSwapchain> {
        let size = self.canvas.get_size().cast().unwrap();
        let (sender, receiver) = webgpu_channel().unwrap();
        let format = WebGpuFormat::R8G8B8A8_SRGB;
        let msg = WebGpuMsg::BuildSwapchain {
            gpu_id: queue.gpu_id(),
            format: WebGpuDevice::map_format(format),
            size,
            result: sender,
        };
        self.sender.send(msg).unwrap();
        let swapchain = receiver.recv().unwrap();

        WebGpuSwapchain::new(&self.global(), self.sender.clone(), format, size, swapchain)
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
