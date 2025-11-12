package main

import (
  "errors"
  "log/slog"
  "net/http"
  "os"

  "github.com/brooknullsh/railway-playground/internal/handler"
  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/labstack/echo/v4"
)

// See: https://github.com/dunamismax/go-web-server

func main() {
  options := slog.HandlerOptions{Level: slog.LevelDebug}
  textHandler := slog.NewTextHandler(os.Stdout, &options)

  // NOTE: Can't use the default handler with a custom level. So we're left to
  // customising the text handler.
  logger := slog.New(textHandler)
  slog.SetDefault(logger)

  store, err := store.NewAndConnect()
  if err != nil {
    slog.Error("[STORE] " + err.Error())
    os.Exit(1)
  }

  defer store.Pool.Close()
  app := echo.New()

  handlers := handler.NewWithState(store)
  handlers.RegisterRoutes(app)

  var port string
  if value, ok := os.LookupEnv("PORT"); ok {
    port = ":" + value
  } else {
    port = ":8080"
  }

  slog.Info("starting server...", "port", port)
  if err := app.Start(port); err != nil && !errors.Is(err, http.ErrServerClosed) {
    slog.Error("[APP] " + err.Error())
    os.Exit(1)
  }
}
