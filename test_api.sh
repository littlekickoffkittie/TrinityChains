#!/bin/bash

# TrinityChain Dashboard API Tests
# Comprehensive test suite to diagnose connectivity issues

echo "=========================================="
echo "🔍 TrinityChain Dashboard API Tests"
echo "=========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

TEST_PASSED=0
TEST_FAILED=0

# Function to test an endpoint
test_endpoint() {
    local endpoint=$1
    local description=$2
    local method=${3:-GET}
    
    echo -e "${BLUE}Testing:${NC} $description"
    echo "  Endpoint: $endpoint"
    echo "  Method: $method"
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s -w "\n%{http_code}" "http://localhost:8000$endpoint")
    else
        response=$(curl -s -X POST -H "Content-Type: application/json" -w "\n%{http_code}" "http://localhost:8000$endpoint")
    fi
    
    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)
    
    echo "  Response Code: $http_code"
    
    if [ "$http_code" = "200" ] || [ "$http_code" = "404" ]; then
        if [ -z "$body" ]; then
            echo -e "  ${YELLOW}⚠ Empty response${NC}"
        else
            echo -e "  ${GREEN}✓ Response received${NC}"
            echo "  Body: $(echo $body | head -c 100)..."
        fi
        echo -e "  ${GREEN}✓ PASS${NC}"
        ((TEST_PASSED++))
    else
        echo -e "  ${RED}✗ FAIL${NC}"
        echo "  Error: $body"
        ((TEST_FAILED++))
    fi
    echo ""
}

# Test 1: Server connectivity
echo -e "${BLUE}=== Test 1: Server Connectivity ===${NC}"
echo "Checking if API server is running on port 8000..."

if nc -z localhost 8000 2>/dev/null; then
    echo -e "${GREEN}✓ Server is running on port 8000${NC}"
    ((TEST_PASSED++))
else
    echo -e "${RED}✗ Server is NOT running on port 8000${NC}"
    echo "Start the server with: cargo run --release --bin trinity-api"
    ((TEST_FAILED++))
    exit 1
fi
echo ""

# Test 2: Blockchain Endpoints
echo -e "${BLUE}=== Test 2: Blockchain Endpoints ===${NC}"
test_endpoint "/api/blockchain/height" "Get blockchain height"
test_endpoint "/api/blockchain/blocks?page=0&limit=10" "Get blocks (paginated)"
test_endpoint "/api/blockchain/stats" "Get blockchain stats"

# Test 3: Transaction Endpoints
echo -e "${BLUE}=== Test 3: Transaction Endpoints ===${NC}"
test_endpoint "/api/mempool" "Get mempool transactions"

# Test 4: Mining Endpoints
echo -e "${BLUE}=== Test 4: Mining Endpoints ===${NC}"
test_endpoint "/api/mining/status" "Get mining status"

# Test 5: Network Endpoints
echo -e "${BLUE}=== Test 5: Network Endpoints ===${NC}"
test_endpoint "/api/network/peers" "Get network peers"
test_endpoint "/api/network/info" "Get network info"

# Test 6: System Endpoints
echo -e "${BLUE}=== Test 6: System Endpoints ===${NC}"
test_endpoint "/health" "Health check"
test_endpoint "/stats" "API statistics"

# Test 7: Detailed API Response
echo -e "${BLUE}=== Test 7: Detailed Blockchain Stats ===${NC}"
echo "Fetching detailed stats..."
stats=$(curl -s http://localhost:8000/api/blockchain/stats)
echo "Response:"
echo "$stats" | python3 -m json.tool 2>/dev/null || echo "$stats"
echo ""

# Test 8: Dashboard Server Check
echo -e "${BLUE}=== Test 8: Dashboard Server ===${NC}"
if nc -z localhost 5173 2>/dev/null; then
    echo -e "${GREEN}✓ Dashboard is running on port 5173${NC}"
    ((TEST_PASSED++))
else
    echo -e "${RED}✗ Dashboard is NOT running on port 5173${NC}"
    echo "Start dashboard with: cd dashboard && npm run dev"
    ((TEST_FAILED++))
fi
echo ""

# Test 9: CORS Headers
echo -e "${BLUE}=== Test 9: CORS Configuration ===${NC}"
echo "Checking CORS headers..."
headers=$(curl -s -i http://localhost:8000/api/blockchain/stats 2>/dev/null | grep -i "access-control")
if [ -z "$headers" ]; then
    echo -e "${YELLOW}⚠ No CORS headers found${NC}"
else
    echo -e "${GREEN}✓ CORS headers present:${NC}"
    echo "$headers"
fi
echo ""

# Test 10: API Response Format
echo -e "${BLUE}=== Test 10: API Response Format ===${NC}"
echo "Testing blockchain/stats response format..."
response=$(curl -s http://localhost:8000/api/blockchain/stats)
if echo "$response" | python3 -m json.tool >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Valid JSON response${NC}"
    ((TEST_PASSED++))
else
    echo -e "${RED}✗ Invalid JSON response${NC}"
    echo "Response: $response"
    ((TEST_FAILED++))
fi
echo ""

# Test 11: Create Wallet
echo -e "${BLUE}=== Test 11: Create Wallet ===${NC}"
echo "Testing wallet creation..."
wallet_response=$(curl -s -X POST http://localhost:8000/api/wallet/create)
if echo "$wallet_response" | python3 -m json.tool >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Wallet creation endpoint working${NC}"
    echo "Response: $wallet_response"
    ((TEST_PASSED++))
else
    echo -e "${YELLOW}⚠ Wallet endpoint response: $wallet_response${NC}"
fi
echo ""

# Summary
echo -e "${BLUE}=========================================="
echo "📊 Test Summary"
echo "==========================================${NC}"
echo -e "Passed: ${GREEN}$TEST_PASSED${NC}"
echo -e "Failed: ${RED}$TEST_FAILED${NC}"
echo ""

if [ $TEST_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    echo ""
    echo "Dashboard should be working. Try:"
    echo "1. Refresh the browser (http://localhost:5173)"
    echo "2. Check browser console for errors (F12)"
    echo "3. Check DevTools Network tab for failed requests"
    exit 0
else
    echo -e "${RED}✗ Some tests failed. Check output above.${NC}"
    exit 1
fi
