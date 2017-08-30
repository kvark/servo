/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
 
typedef unsigned long WebGpuSemaphore;
typedef unsigned long WebGpuFormat;
typedef unsigned long WebGpuRenderTargetView;
typedef unsigned long WebGpuDepthStencilView;

dictionary WebGpuAttachmentDesc {
	required WebGpuFormat format;
};

dictionary WebGpuSubpassDesc {
	sequence<unsigned long> colorAttachments;
};

enum WebGpuFenceWait {
	"Any",
	"All",
};

interface WebGpuDevice {
	readonly attribute WebGpuCommandQueue generalQueue; //TODO: FrozenArray<>

	WebGpuFence createFence(boolean set);
	void resetFences(sequence<WebGpuFence> fences);
	void waitForFences(
		sequence<WebGpuFence> fences,
		WebGpuFenceWait mode,
		unsigned long timeout
	);

	WebGpuRenderpass createRenderpass(
		sequence<WebGpuAttachmentDesc> attachments,
		sequence<WebGpuSubpassDesc> subpasses
	);

	WebGpuFramebuffer createFramebuffer(
		WebGpuRenderpass renderpass,
		sequence<WebGpuRenderTargetView> colors,
		WebGpuDepthStencilView? depth_stencil
	);

	WebGpuRenderTargetView viewImageAsRenderTarget(WebGpuImage image);
};
