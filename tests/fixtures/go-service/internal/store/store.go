// Package store guarda pedidos em memória.
package store

import (
	"errors"
	"sync"
)

// ErrNotFound: func Fake() {} não é declaração.
var ErrNotFound = errors.New("pedido não encontrado")

type Order struct {
	ID    string
	Total int64
}

type Store struct {
	mu     sync.Mutex
	orders memoryIndex
}

func New() *Store {
	return &Store{orders: memoryIndex{}}
}

func (s *Store) Save(o Order) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.orders[o.ID] = o
	return nil
}
