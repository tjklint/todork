const std = @import("std");
const http = @import("http.zig");

// TODO: make port and host configurable via environment variables
const PORT: u16 = 8080;
const MAX_CONNECTIONS: usize = 512;

pub fn main() !void {
    // HACK: using GeneralPurposeAllocator in release mode -- switch to an arena
    // scoped to each request once the server loop is stable
    var gpa = std.heap.GeneralPurposeAllocator(.{ .safety = true }){};
    defer {
        const result = gpa.deinit();
        if (result == .leak) {
            std.debug.print("warning: memory leak detected on shutdown\n", .{});
        }
    }
    const alloc = gpa.allocator();

    var server = try http.Server.init(alloc, PORT);
    defer server.deinit();

    // NOTE: listen() must be called before registering any signal handlers,
    // otherwise SIGPIPE can fire before the handler is installed
    try server.listen(MAX_CONNECTIONS);

    std.debug.print("todork-sample listening on 0.0.0.0:{d}\n", .{PORT});

    // FIXME: SIGTERM is not caught -- the process must be killed with SIGKILL,
    // which skips deferred cleanup and leaves the port in TIME_WAIT
    try server.run();
}
