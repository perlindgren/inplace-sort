TYPST = typst
DRAWIO = drawio
SRC_DIR = doc
BUILD_DIR = build
DIAGRAMS_DIR = figs
BUILD_DIAGRAMS_DIR = $(BUILD_DIR)/$(DIAGRAMS_DIR)
MAIN_TYP = $(SRC_DIR)/rtas.typ
OUTPUT_PDF = $(BUILD_DIR)/main.pdf
ROOT_DIR = .

# Find all .drawio files in the diagrams directory
DIAGRAM_FILES = $(wildcard $(DIAGRAMS_DIR)/*.drawio)

all: $(OUTPUT_PDF)
diagrams: $(BUILD_DIAGRAMS_DIR)/built

watch: $(OUTPUT_PDF)
	$(TYPST) watch --root $(ROOT_DIR) $(MAIN_TYP) $(OUTPUT_PDF)

# Build the main PDF
$(OUTPUT_PDF): $(MAIN_TYP) $(BUILD_DIAGRAMS_DIR)/built
	mkdir -p $(BUILD_DIR)
	$(TYPST) compile --root $(ROOT_DIR) $(MAIN_TYP) $(OUTPUT_PDF)

# Convert .drawio files to .pdf
$(BUILD_DIAGRAMS_DIR)/built: $(DIAGRAM_FILES) | $(BUILD_DIAGRAMS_DIR)
	$(DRAWIO) $(DIAGRAMS_DIR) --export --format pdf --crop --output $(BUILD_DIAGRAMS_DIR) 2>/dev/null
	touch $@

# Ensure the build/diagrams directory exists
$(BUILD_DIAGRAMS_DIR):
	mkdir -p $(BUILD_DIAGRAMS_DIR)

# Clean build artifacts
clean:
	rm -rf $(BUILD_DIR)

.PHONY: all diagrams clean watch