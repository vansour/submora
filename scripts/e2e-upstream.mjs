import { createServer } from "node:http";

const port = Number(process.env.PLAYWRIGHT_UPSTREAM_PORT ?? "19081");

const server = createServer((request, response) => {
  if (request.url === "/healthz") {
    response.writeHead(200, { "content-type": "text/plain; charset=utf-8" });
    response.end("ok");
    return;
  }

  if (request.url === "/feed") {
    response.writeHead(200, { "content-type": "text/plain; charset=utf-8" });
    response.end("https://one.example/feed\nhttps://two.example/feed\n");
    return;
  }

  response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
  response.end("not found");
});

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => {
    server.close(() => process.exit(0));
  });
}

server.listen(port, "127.0.0.1");
