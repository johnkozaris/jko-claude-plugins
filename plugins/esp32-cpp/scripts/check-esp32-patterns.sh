#!/bin/bash
# Post-write check for common ESP32 anti-patterns
# Runs after Write/Edit tool on .cpp, .c, .h, .hpp files

# Get the file path from the tool input
if command -v jq &>/dev/null; then
    FILE_PATH=$(echo "$CLAUDE_TOOL_INPUT" | jq -r '.file_path // empty' 2>/dev/null)
else
    FILE_PATH=$(echo "$CLAUDE_TOOL_INPUT" | grep -o '"file_path":"[^"]*"' | head -1 | cut -d'"' -f4)
fi

if [[ -z "$FILE_PATH" ]]; then
    exit 0
fi

# Only check C/C++ files
if [[ ! "$FILE_PATH" =~ \.(cpp|c|h|hpp|cc|cxx)$ ]]; then
    exit 0
fi

# Only check if file exists
if [[ ! -f "$FILE_PATH" ]]; then
    exit 0
fi

WARNINGS=""

# Check for ISR functions without IRAM_ATTR
if grep -n 'void.*_isr\|void.*_handler\|void.*ISR' "$FILE_PATH" | grep -v 'IRAM_ATTR' | grep -v '//' | head -3 | grep -q .; then
    WARNINGS="$WARNINGS\n[AP-02] Possible ISR function without IRAM_ATTR. ISRs must be in IRAM."
fi

# Check for printf/ESP_LOG in functions marked IRAM_ATTR
if grep -A 5 'IRAM_ATTR' "$FILE_PATH" | grep -q 'ESP_LOG\|printf\|malloc\|free\b'; then
    WARNINGS="$WARNINGS\n[AP-01] printf/ESP_LOG/malloc found near IRAM_ATTR function. These are NOT ISR-safe."
fi

# Check for delay() in non-Arduino context (should be vTaskDelay)
if grep -n 'delay(' "$FILE_PATH" | grep -v 'vTaskDelay\|esp_rom_delay\|ets_delay_us\|//' | head -3 | grep -q .; then
    WARNINGS="$WARNINGS\n[AP-11] delay() found. Use vTaskDelay(pdMS_TO_TICKS(ms)) in FreeRTOS tasks."
fi

# Check for hardcoded credentials
if grep -in '#define.*\(PASS\|PASSWORD\|SECRET\|KEY\|TOKEN\).*"' "$FILE_PATH" | head -1 | grep -q .; then
    WARNINGS="$WARNINGS\n[AP-16] SECURITY: Possible hardcoded credentials. Store in NVS, not source code."
fi

# Check for volatile used with shared data patterns
if grep -n 'volatile.*shared\|volatile.*global\|volatile.*counter' "$FILE_PATH" | head -1 | grep -q .; then
    WARNINGS="$WARNINGS\n[AP-09] volatile may be used for synchronization. Use FreeRTOS primitives (mutex/queue/tasknotify) instead."
fi

if [[ -n "$WARNINGS" ]]; then
    echo "{\"continue\": true, \"suppressOutput\": false, \"systemMessage\": \"ESP32 pattern check on $(basename $FILE_PATH):$(echo -e $WARNINGS)\"}"
else
    echo '{"continue": true, "suppressOutput": true}'
fi
