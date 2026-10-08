package main

import (
	"context"
	_ "embed"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"
)

//go:embed assets/message.txt
var message []byte

func main() {
	if len(os.Args) > 1 && os.Args[1] == "--self-check" {
		if err := json.NewEncoder(os.Stdout).Encode(map[string]any{"message": string(message), "args": os.Args[2:]}); err != nil {
			panic(err)
		}
		return
	}
	http.HandleFunc("/argv", func(w http.ResponseWriter, r *http.Request) {
		json.NewEncoder(w).Encode(os.Args[1:])
	})
	http.HandleFunc("/ready", func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprint(w, "ready")
	})
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Write(message)
	})
	server := &http.Server{Addr: "0.0.0.0:" + os.Getenv("PORT")}
	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGTERM)
	defer stop()
	closed := make(chan struct{})
	go func() {
		<-ctx.Done()
		deadline, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		if err := server.Shutdown(deadline); err != nil {
			panic(err)
		}
		close(closed)
	}()
	if err := server.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
		panic(err)
	}
	<-closed
}
