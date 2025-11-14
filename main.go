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
  text := err.Error()
  slog.Error(text)

  os.Exit(1)
}

func setLogger() {
  options := slog.HandlerOptions{Level: slog.LevelDebug}
  overwrite := slog.NewTextHandler(os.Stdout, &options)

  logger := slog.New(overwrite)
  slog.SetDefault(logger)
}

func buildPort() (port string) {
  if value, exists := os.LookupEnv("PORT"); exists {
    port = ":" + value
  } else {
    port = ":8080"
  }

  return
}

func setup() (*store.Store, *echo.Echo) {
  store, err := store.NewAndConnect()
  if err != nil {
    abort(err)
  }

  app := echo.New()
  handler.InitialiseWithState(app, store)

  return store, app
}

func main() {
  setLogger()

  store, app := setup()
  defer store.Pool.Close()
  port := buildPort()

  if err := app.Start(port); err != nil && !errors.Is(err, http.ErrServerClosed) {
    abort(err)
  }
}
