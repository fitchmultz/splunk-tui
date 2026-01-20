#!/usr/bin/env bash
# Test script for live Splunk server verification
# This script tests the splunk CLI against a live Splunk server

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=========================================="
echo "Splunk TUI Live Server Test"
echo "=========================================="
echo ""

# Check if splunk CLI is installed
if ! command -v splunk &> /dev/null; then
    echo -e "${RED}Error: splunk CLI not found${NC}"
    echo "Please run: make install"
    exit 1
fi

# Test 1: Search command with basic query
echo -e "${YELLOW}Test 1: Search command with basic query${NC}"
if splunk search "index=main | head 1" --wait --count 1 &> /dev/null; then
    echo -e "${GREEN}✓ Search command succeeded${NC}"
else
    echo -e "${RED}✗ Search command failed${NC}"
    exit 1
fi
echo ""

# Test 2: List indexes
echo -e "${YELLOW}Test 2: List indexes${NC}"
if splunk indexes --detailed --count 10 &> /dev/null; then
    echo -e "${GREEN}✓ List indexes succeeded${NC}"
else
    echo -e "${RED}✗ List indexes failed${NC}"
    exit 1
fi
echo ""

# Test 3: List jobs
echo -e "${YELLOW}Test 3: List jobs${NC}"
if splunk jobs --list --count 10 &> /dev/null; then
    echo -e "${GREEN}✓ List jobs succeeded${NC}"
else
    echo -e "${RED}✗ List jobs failed${NC}"
    exit 1
fi
echo ""

# Test 4: Cluster info (may fail on standalone instance)
echo -e "${YELLOW}Test 4: Get cluster info${NC}"
if splunk cluster &> /dev/null; then
    echo -e "${GREEN}✓ Cluster info succeeded (clustered instance)${NC}"
else
    echo -e "${YELLOW}⚠ Cluster info failed (expected on standalone instance)${NC}"
fi
echo ""

# Test 5: Search with output formats
echo -e "${YELLOW}Test 5: Search with JSON output${NC}"
if splunk search "index=main | head 1" --output json --count 1 &> /dev/null; then
    echo -e "${GREEN}✓ Search with JSON output succeeded${NC}"
else
    echo -e "${RED}✗ Search with JSON output failed${NC}"
    exit 1
fi
echo ""

echo "=========================================="
echo -e "${GREEN}All live server tests passed!${NC}"
echo "=========================================="
