package api

import (
	"encoding/json"
	"net/http"

	"github.com/acme/orders/internal/store"
)

const (
	DefaultLimit = 50
	maxBody      = 1 << 20
)

type Handler struct {
	store *store.Store
}

func NewHandler(s *store.Store) *Handler {
	return &Handler{store: s}
}

func (h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	o, err := h.store.Find(r.URL.Query().Get("id"))
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}
	_ = json.NewEncoder(w).Encode(o)
}
