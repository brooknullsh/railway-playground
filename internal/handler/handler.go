package handler

import (
  "net/http"

  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/gofiber/fiber/v3"
)

const StoreKey = "store"

func Index(ctx fiber.Ctx) error {
  store, cast := fiber.GetState[*store.Store](ctx.App().State(), StoreKey)
  if !cast {
    return ctx.SendStatus(http.StatusInternalServerError)
  }

  user, code := store.GetUserByName(ctx, "Alice")
  if code != http.StatusOK {
    return ctx.SendStatus(code)
  }

  return ctx.SendString(user.FirstName)
}
