/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

typedef unsigned long WebGpuAccess;
enum WebGpuImageLayout {
	"General",
	"ColorAttachmentOptimal",
	"DepthStencilAttachmentOptimal",
	"DepthStencilReadOnlyOptimal",
	"ShaderReadOnlyOptimal",
	"TransferSrcOptimal",
	"TransferDstOptimal",
	"Undefined",
	"Preinitialized",
	"Present",
};

dictionary WebGpuBufferState {
	required WebGpuAccess access;
};
dictionary WebGpuImageState {
	required WebGpuAccess access;
	required WebGpuImageLayout layout;
};

dictionary WebGpuBufferBarrier {
	required WebGpuBufferState stateSrc;
	required WebGpuBufferState stateDst;
	required WebGpuBuffer target;
	//TODO: offset/size
};

dictionary WebGpuImageBarrier {
	required WebGpuImageState stateSrc;
	required WebGpuImageState stateDst;
	required WebGpuImage target;
	//TODO: subresource range
};

typedef unsigned long WebGpuRectangle;
typedef unsigned long WebGpuClearValue;

interface WebGpuCommandBuffer {
	const WebGpuAccess ACCESS_INDEX_BUFFER_READ      = 0x1;
	const WebGpuAccess ACCESS_VERTEX_BUFFER_READ     = 0x2;
	const WebGpuAccess ACCESS_CONSTANT_BUFFER_READ   = 0x4;
	const WebGpuAccess ACCESS_INDIRECT_COMMAND_READ  = 0x8;
	const WebGpuAccess ACCESS_RENDER_TARGET_CLEAR    = 0x20;
	const WebGpuAccess ACCESS_RESOLVE_SRC            = 0x100;
	const WebGpuAccess ACCESS_RESOLVE_DST            = 0x200;
	const WebGpuAccess ACCESS_COLOR_ATTACHMENT_READ  = 0x1;
	const WebGpuAccess ACCESS_COLOR_ATTACHMENT_WRITE = 0x2;
	const WebGpuAccess ACCESS_TRANSFER_READ          = 0x4;
	const WebGpuAccess ACCESS_TRANSFER_WRITE         = 0x8;
	const WebGpuAccess ACCESS_SHADER_READ            = 0x10;

	WebGpuSubmit finish();
	void pipelineBarrier(
		sequence<WebGpuBufferBarrier> buffers,
		sequence<WebGpuImageBarrier> images
	);

	void beginRenderpass(
		WebGpuRenderpass renderpass,
		WebGpuFramebuffer framebuffer,
		WebGpuRectangle rect,
		sequence<WebGpuClearValue> clearValues
	);

	void endRenderpass();
};
