#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

SELECTED_OPENCODE=0
SELECTED_CLAUDE=0
SELECTED_COPILOT=0

print_banner() {
    echo ""
    echo -e "${CYAN}${BOLD}╔══════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}${BOLD}║        AI Coding Space Setup             ║${NC}"
    echo -e "${CYAN}${BOLD}║        crypto-pocket-butler              ║${NC}"
    echo -e "${CYAN}${BOLD}╚══════════════════════════════════════════╝${NC}"
    echo ""
}

print_platforms() {
    echo -e "${BOLD}Select AI platforms to set up${NC}"
    echo -e "${YELLOW}Enter numbers separated by spaces (e.g., 1 3):${NC}"
    echo ""
    echo -e "  ${GREEN}1)${NC} Opencode"
    echo -e "  ${GREEN}2)${NC} Claude"
    echo -e "  ${GREEN}3)${NC} GitHub Copilot"
    echo ""
}

ask_platforms() {
    while true; do
        print_platforms
        read -r -p "  Your choices: " choices
        echo ""

        SELECTED_OPENCODE=0
        SELECTED_CLAUDE=0
        SELECTED_COPILOT=0
        VALID=1

        for choice in $choices; do
            case "$choice" in
                1) SELECTED_OPENCODE=1 ;;
                2) SELECTED_CLAUDE=1 ;;
                3) SELECTED_COPILOT=1 ;;
                *)
                    echo -e "${RED}Invalid choice: $choice. Please enter numbers 1-3.${NC}"
                    VALID=0
                    break
                    ;;
            esac
        done

        if [ "$VALID" -eq 0 ]; then
            continue
        fi

        if [ "$SELECTED_OPENCODE" -eq 0 ] && [ "$SELECTED_CLAUDE" -eq 0 ] && [ "$SELECTED_COPILOT" -eq 0 ]; then
            echo -e "${RED}No platforms selected. Please enter at least one number.${NC}"
            continue
        fi

        break
    done
}

confirm_overwrite() {
    local file="$1"
    if [ -f "$file" ]; then
        echo -e "${YELLOW}  Warning: $(basename "$file") already exists at $(dirname "$file")${NC}"
        read -r -p "  Overwrite? [y/N] " answer
        case "$answer" in
            [yY]*) return 0 ;;
            *) return 1 ;;
        esac
    fi
    return 0
}

copy_skills() {
    local target_dir="$1"
    local skill_count
    skill_count=$(find "$SCRIPT_DIR/skills" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')

    if [ "$skill_count" -eq 0 ]; then
        echo -e "  ${YELLOW}⊘${NC} No skills found in $SCRIPT_DIR/skills/"
        return
    fi

    for skill_dir in "$SCRIPT_DIR/skills"/*/; do
        [ -d "$skill_dir" ] || continue
        local skill_name
        skill_name="$(basename "$skill_dir")"
        mkdir -p "$target_dir/$skill_name"
        cp "$skill_dir/SKILL.md" "$target_dir/$skill_name/SKILL.md"
    done

    echo -e "  ${GREEN}✓${NC} Copied $skill_count skill(s) to ${target_dir#$PROJECT_ROOT/}"
}

setup_opencode() {
    echo -e "${BLUE}Setting up Opencode...${NC}"

    local target_agents="$PROJECT_ROOT/AGENTS.md"
    if confirm_overwrite "$target_agents"; then
        cp "$SCRIPT_DIR/AGENTS.md" "$target_agents"
        echo -e "  ${GREEN}✓${NC} Created AGENTS.md"
    else
        echo -e "  ${YELLOW}⊘${NC} Skipped AGENTS.md"
    fi

    copy_skills "$PROJECT_ROOT/.opencode/skills"
}

setup_claude() {
    echo -e "${BLUE}Setting up Claude...${NC}"

    local target_claude="$PROJECT_ROOT/CLAUDE.md"
    if confirm_overwrite "$target_claude"; then
        cp "$SCRIPT_DIR/CLAUDE.md" "$target_claude"
        echo -e "  ${GREEN}✓${NC} Created CLAUDE.md"
    else
        echo -e "  ${YELLOW}⊘${NC} Skipped CLAUDE.md"
    fi

    copy_skills "$PROJECT_ROOT/.claude/skills"
}

setup_copilot() {
    echo -e "${BLUE}Setting up GitHub Copilot...${NC}"

    local target_agents="$PROJECT_ROOT/AGENTS.md"
    if [ "$SELECTED_OPENCODE" -eq 1 ]; then
        echo -e "  ${GREEN}✓${NC} AGENTS.md already set up by Opencode"
    else
        if confirm_overwrite "$target_agents"; then
            cp "$SCRIPT_DIR/AGENTS.md" "$target_agents"
            echo -e "  ${GREEN}✓${NC} Created AGENTS.md"
        else
            echo -e "  ${YELLOW}⊘${NC} Skipped AGENTS.md"
        fi
    fi

    copy_skills "$PROJECT_ROOT/.agents/skills"
}

print_summary() {
    echo ""
    echo -e "${CYAN}${BOLD}══════════════════════════════════════════${NC}"
    echo -e "${CYAN}${BOLD}  Setup Complete!${NC}"
    echo -e "${CYAN}${BOLD}══════════════════════════════════════════${NC}"
    echo ""

    echo -e "  Configured platforms:"
    [ "$SELECTED_OPENCODE" -eq 1 ] && echo -e "    ${GREEN}●${NC} Opencode         → AGENTS.md + .opencode/skills/"
    [ "$SELECTED_CLAUDE" -eq 1 ]   && echo -e "    ${GREEN}●${NC} Claude           → CLAUDE.md + .claude/skills/"
    [ "$SELECTED_COPILOT" -eq 1 ]  && echo -e "    ${GREEN}●${NC} GitHub Copilot   → AGENTS.md + .agents/skills/"

    echo ""
    echo -e "  ${BOLD}Files created:${NC}"
    [ "$SELECTED_OPENCODE" -eq 1 ] || [ "$SELECTED_COPILOT" -eq 1 ] && echo -e "    AGENTS.md"
    [ "$SELECTED_CLAUDE" -eq 1 ]   && echo -e "    CLAUDE.md"
    [ "$SELECTED_OPENCODE" -eq 1 ] && echo -e "    .opencode/skills/*/SKILL.md"
    [ "$SELECTED_CLAUDE" -eq 1 ]   && echo -e "    .claude/skills/*/SKILL.md"
    [ "$SELECTED_COPILOT" -eq 1 ]  && echo -e "    .agents/skills/*/SKILL.md"

    echo ""
    echo -e "  ${YELLOW}Tip:${NC} Run this script again to reconfigure or add more platforms."
    echo ""
}

verify_project_root() {
    if [ ! -d "$PROJECT_ROOT/.git" ]; then
        echo -e "${RED}Error: Could not find project root (no .git directory at $PROJECT_ROOT)${NC}"
        exit 1
    fi

    if [ ! -f "$SCRIPT_DIR/AGENTS.md" ]; then
        echo -e "${RED}Error: AGENTS.md template not found in $SCRIPT_DIR${NC}"
        exit 1
    fi

    if [ ! -f "$SCRIPT_DIR/CLAUDE.md" ]; then
        echo -e "${RED}Error: CLAUDE.md template not found in $SCRIPT_DIR${NC}"
        exit 1
    fi
}

verify_project_root
print_banner
ask_platforms

echo ""
[ "$SELECTED_OPENCODE" -eq 1 ] && setup_opencode
echo ""
[ "$SELECTED_CLAUDE" -eq 1 ]   && setup_claude
echo ""
[ "$SELECTED_COPILOT" -eq 1 ]  && setup_copilot

print_summary
