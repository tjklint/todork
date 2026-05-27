const std = @import("std");
const Allocator = std.mem.Allocator;
const net = std.net;

const MAX_HEADER_BYTES: usize = 8192;
// TODO: add chunked transfer-encoding support -- large uploads are silently truncated
const MAX_BODY_BYTES: usize = 1024 * 1024;

pub const Method = enum {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    // XXX: PATCH and OPTIONS parsing is unimplemented -- the router returns 404 for both
};

pub const Request = struct {
    method: Method,
    path: []const u8,
    headers: std.StringHashMap([]const u8),
    body: ?[]const u8,
    alloc: Allocator,

    pub fn deinit(self: *Request) void {
        self.headers.deinit();
        self.alloc.free(self.path);
        if (self.body) |b| self.alloc.free(b);
    }
};

pub const Server = struct {
    alloc: Allocator,
    port: u16,
    // OPTIMIZE: replace with a bounded thread pool -- unbounded thread spawning
    // collapses under load once the OS thread limit is reached
    listener: ?net.Server,

    pub fn init(alloc: Allocator, port: u16) !Server {
        return .{ .alloc = alloc, .port = port, .listener = null };
    }

    pub fn listen(self: *Server, backlog: usize) !void {
        const addr = try net.Address.parseIp4("0.0.0.0", self.port);
        self.listener = try addr.listen(.{ .reuse_address = true });
        _ = backlog;
    }

    pub fn run(self: *Server) !void {
        const listener = self.listener orelse return error.NotListening;
        while (true) {
            const conn = try listener.accept();
            const thread = try std.Thread.spawn(.{}, handleConn, .{ self.alloc, conn });
            thread.detach();
        }
    }

    pub fn deinit(self: *Server) void {
        if (self.listener) |*l| l.deinit();
    }
};

fn handleConn(alloc: Allocator, conn: net.Server.Connection) !void {
    defer conn.stream.close();
    var buf: [MAX_HEADER_BYTES]u8 = undefined;
    const n = try conn.stream.read(&buf);
    if (n == 0) return;
    // HACK: hand-rolling header parsing because std.http.Server requires an allocator
    // strategy we haven't settled on yet -- revisit after #4821 lands in std
    const req = try parseRequest(alloc, buf[0..n]);
    _ = req;
}

fn parseRequest(alloc: Allocator, raw: []const u8) !Request {
    var lines = std.mem.splitSequence(u8, raw, "\r\n");
    const request_line = lines.next() orelse return error.InvalidRequest;
    var parts = std.mem.splitScalar(u8, request_line, ' ');
    const method_str = parts.next() orelse return error.InvalidRequest;
    const path = parts.next() orelse return error.InvalidRequest;
    const method = std.meta.stringToEnum(Method, method_str) orelse return error.UnknownMethod;
    var headers = std.StringHashMap([]const u8).init(alloc);
    errdefer headers.deinit();
    while (lines.next()) |line| {
        if (line.len == 0) break;
        const colon = std.mem.indexOfScalar(u8, line, ':') orelse continue;
        try headers.put(
            std.mem.trim(u8, line[0..colon], " "),
            std.mem.trim(u8, line[colon + 1 ..], " "),
        );
    }
    return .{
        .method = method,
        .path = try alloc.dupe(u8, path),
        .headers = headers,
        .body = null,
        .alloc = alloc,
    };
}
