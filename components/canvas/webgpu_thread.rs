/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::*;
use euclid::Size2D;
use std::thread;

pub struct WebGpuThread {
    /// Id generator for new WebGpuContexts.
    next_webgpu_id: usize,
}

impl WebGpuThread {
    fn new() -> Self {
        WebGpuThread {
            next_webgpu_id: 0,
        }
    }

    /// Creates a new `WebGpuThread` and returns a Sender to
    /// communicate with it.
    pub fn start() -> WebGpuSender<WebGpuMsg> {
        let (sender, receiver) = webgpu_channel::<WebGpuMsg>().unwrap();
        let result = sender.clone();
        thread::Builder::new().name("WebGpuThread".to_owned()).spawn(move || {
            let mut renderer = WebGpuThread::new();
            let webgpu_chan = sender;
            loop {
                let msg = receiver.recv().unwrap();
                let exit = renderer.handle_msg(msg, &webgpu_chan);
                if exit {
                    return;
                }
            }
        }).expect("Thread spawning failed");

        result
    }

    /// Handles a generic WebGpuMsg message
    #[inline]
    fn handle_msg(&mut self, msg: WebGpuMsg, webgpu_chan: &WebGpuChan) -> bool {
        match msg {
            WebGpuMsg::CreateContext { size, num_frames, sender } => {
                let result = self.create_webgpu_context(size, num_frames);
                sender.send(result.map(|id|
                    WebGpuInit {
                        sender: WebGpuMsgSender::new(id, webgpu_chan.clone()),
                    }
                )).unwrap();
            }
            WebGpuMsg::Exit => {
                return true;
            }
        }

        false
    }

    /// Creates a new WebGpuContext
    fn create_webgpu_context(&mut self,
        size: Size2D<i32>,
        num_frames: u8,
    ) -> Result<WebGpuContextId, String>
    {
        let id = WebGpuContextId(self.next_webgpu_id);
        self.next_webgpu_id += 1;
        Ok(id)
    }
}

/// WebGPU Threading API entry point that lives in the constellation.
pub struct WebGpuThreads(WebGpuSender<WebGpuMsg>);

impl WebGpuThreads {
    /// Creates a new WebGpuThreads object
    pub fn new() -> Self {
        // This implementation creates a single `WebGpuThread` for all the pipelines.
        WebGpuThreads(WebGpuThread::start())
    }

    /// Gets the WebGpuThread handle for each script pipeline.
    pub fn pipeline(&self) -> WebGpuPipeline {
        // This mode creates a single thread, so the existing WebGpuChan is just cloned.
        WebGpuPipeline(self.0.clone())
    }

    /// Sends a exit message to close the WebGpuThreads and release all WebGpuContexts.
    pub fn exit(&self) -> Result<(), &'static str> {
        self.0.send(WebGpuMsg::Exit).map_err(|_| "Failed to send Exit message")
    }
}
