package store

import "testing"

func TestSaveAndFind(t *testing.T) {
	s := New()
	_ = s.Save(Order{ID: "a1", Total: 10})
	if _, err := s.Find("a1"); err != nil {
		t.Fatal(err)
	}
}
