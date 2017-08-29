/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
 
typedef unsigned long WebGpuSemaphore;
typedef unsigned long WebGpuFence;
typedef unsigned long WebGpuRenderpass;
typedef unsigned long WebGpuFramebuffer;
typedef unsigned long WebGpuFormat;
typedef unsigned long WebGpuRenderTargetView;

dictionary WebGpuAttachmentDesc {
	required WebGpuFormat format;
};
dictionary WebGpuSubpassDesc {
	sequence<unsigned long> colorAttachments;
};

interface WebGpuDevice {
	readonly attribute WebGpuCommandQueue generalQueue; //TODO: FrozenArray<>

	WebGpuRenderpass createRenderPass(
		sequence<WebGpuAttachmentDesc> attachments,
		sequence<WebGpuSubpassDesc> subpasses
	);

	WebGpuFramebuffer createFramebuffer(sequence<WebGpuRenderTargetView> colors);
	WebGpuRenderTargetView viewImageAsRenderTarget(WebGpuImage image);
};
