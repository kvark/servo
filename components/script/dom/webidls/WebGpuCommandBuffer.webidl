/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

dictionary WebGpuBufferState {
	required WebGpuBufferAccess access;
};

dictionary WebGpuImageState {
	required WebGpuImageAccess access;
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

dictionary WebGpuRectangle {
	required unsigned long x;
	required unsigned long y;
	required unsigned long width;
	required unsigned long height;
};

enum WebGpuClearValueKind {
	"ColorUint",
	"ColorInt",
	"ColorFloat",
	"DepthStencil",
};

dictionary WebGpuClearValue {
	required WebGpuClearValueKind kind;
	required sequence<float> data;
};

interface WebGpuCommandBuffer {
	void begin();
	void finish();

	void pipelineBarrier(
		WebGpuPipelineStage srcStages,
		WebGpuPipelineStage dstStages,
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
