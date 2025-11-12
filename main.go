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

func abort(err error) {
  errorMsg := err.Error()
  slog.Error(errorMsg)

  os.Exit(1)
}

func setup() (storage *store.Store, app *echo.Echo, port string) {
  options := slog.HandlerOptions{Level: slog.LevelDebug}
  textHandler := slog.NewTextHandler(os.Stdout, &options)

  // Can't use the default handler when specifying a custom level.
  logger := slog.New(textHandler)
  slog.SetDefault(logger)

  storage, err := store.NewAndConnect()
  if err != nil {
    abort(err)
  }

  app = echo.New()
  handlers := handler.NewWithState(storage)
  handlers.RegisterRoutes(app)

  if envPort, exists := os.LookupEnv("PORT"); exists {
    port = ":" + envPort
  } else {
    port = ":8080"
  }

  return
}

func main() {
  store, app, port := setup()
  defer store.Pool.Close()

  slog.Info("starting server...", "port", port)
  if err := app.Start(port); err != nil && !errors.Is(err, http.ErrServerClosed) {
    abort(err)
  }
}
