package handler

import (
  "log/slog"
  "net/http"
  "os"

  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/gofiber/fiber/v3"
  "github.com/gofiber/fiber/v3/middleware/logger"
)

const (
  accTokenKey = "access_secret"
  refTokenKey = "refresh_secret"
)

func Setup(app *fiber.App, store *store.Store) {
  var handler Handler
  handler.store = store

  logger := logger.New()
  accSecret := os.Getenv("ACCESS_SECRET")
  refSecret := os.Getenv("REFRESH_SECRET")

  if accSecret == "" || refSecret == "" {
    slog.Error("missing access and/or refresh secret(s)")
    os.Exit(1)
  }

  app.Use(logger)
  app.Use("/", handler.AuthMiddleware)
  app.State().Set(accTokenKey, accSecret)
  app.State().Set(refTokenKey, refSecret)

  app.Get("/", handler.Index)
  app.Get("/login", handler.Login)
}

type Handler struct {
  store *store.Store
}

func (this *Handler) Index(ctx fiber.Ctx) error {
  user := fiber.MustGetState[UserState](ctx.App().State(), "user")
  slog.Info("found user in state", "user_id", user.Id)
  return ctx.SendStatus(http.StatusOK)
}
