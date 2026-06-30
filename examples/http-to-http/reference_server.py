from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import sys


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/health":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"ok")
            return
        self.send_response(404)
        self.end_headers()

    def do_POST(self):
        if self.path == "/api/invoices":
            self.send_response(201)
            self.send_header("content-type", "application/json")
            self.end_headers()
            self.wfile.write(
                json.dumps(
                    {
                        "id": "inv_123",
                        "amount": "199.90",
                        "currency": "BRL",
                        "status": "open",
                    }
                ).encode()
            )
            return
        self.send_response(404)
        self.end_headers()

    def log_message(self, *_args):
        return


HTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
