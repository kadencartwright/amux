.PHONY: build test install clean

BINARY_NAME := amux
BUILD_DIR := .
INSTALL_DIR := $(HOME)/bin

# Build the binary
build:
	go build -o $(BUILD_DIR)/$(BINARY_NAME) ./cmd/amux

# Run tests
test:
	go test -v ./...

# Build and install to ~/bin
install: build
	@echo "Installing $(BINARY_NAME) to $(INSTALL_DIR)..."
	@mkdir -p $(INSTALL_DIR)
	@cp $(BUILD_DIR)/$(BINARY_NAME) $(INSTALL_DIR)/$(BINARY_NAME)
	@echo "Installation complete!"
	@echo "Make sure $(INSTALL_DIR) is in your PATH"

# Clean build artifacts
clean:
	rm -f $(BUILD_DIR)/$(BINARY_NAME)

# Development helpers
dev: build
	./$(BINARY_NAME) start

init: build
	./$(BINARY_NAME) init
