// Command orders sobe a API de pedidos.
package main

import (
	"fmt"
	"net/http"

	"github.com/acme/orders/internal/api"
	"github.com/acme/orders/internal/store"
)

func main() {
	s := store.New()
	fmt.Println("ouvindo em :8080")
	_ = http.ListenAndServe(":8080", api.NewHandler(s))
}
