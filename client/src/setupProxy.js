const { createProxyMiddleware } = require("http-proxy-middleware");

module.exports = (app) => {
  ["/api", "/connect"].map((path) =>
    app.use(
      path,
      createProxyMiddleware({
        target: "http://localhost:8000",
      })
    )
  );
};
