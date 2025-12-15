#!/usr/bin/env python3
"""
TrinityChain Dashboard API Test Suite
Comprehensive diagnostic tests
"""

import requests
import json
import sys
from datetime import datetime

# Colors
GREEN = '\033[92m'
RED = '\033[91m'
YELLOW = '\033[93m'
BLUE = '\033[94m'
RESET = '\033[0m'

API_URL = "http://localhost:3000"
DASHBOARD_URL = "http://localhost:5173"

def print_header(text):
    print(f"\n{BLUE}{'='*50}{RESET}")
    print(f"{BLUE}{text}{RESET}")
    print(f"{BLUE}{'='*50}{RESET}\n")

def print_test(name):
    print(f"{BLUE}→ {name}{RESET}")

def print_pass(msg):
    print(f"  {GREEN}✓ {msg}{RESET}")

def print_fail(msg):
    print(f"  {RED}✗ {msg}{RESET}")

def print_warn(msg):
    print(f"  {YELLOW}⚠ {msg}{RESET}")

def test_endpoint(path, method="GET", description=""):
    """Test a single API endpoint"""
    print_test(f"{description or path}")
    
    url = f"{API_URL}{path}"
    try:
        if method == "GET":
            response = requests.get(url, timeout=5)
        elif method == "POST":
            response = requests.post(url, timeout=5, json={})
        else:
            print_fail("Unknown method")
            return False
        
        print(f"    URL: {url}")
        print(f"    Status: {response.status_code}")
        
        # Try to parse JSON
        try:
            data = response.json()
            print_pass(f"Valid JSON response")
            print(f"    Data: {json.dumps(data)[:100]}...")
            return True
        except:
            if response.text:
                print_warn(f"Response: {response.text[:100]}")
            else:
                print_fail("Empty response")
            return response.status_code < 500
            
    except requests.exceptions.ConnectionError:
        print_fail("Connection refused")
        return False
    except requests.exceptions.Timeout:
        print_fail("Request timeout")
        return False
    except Exception as e:
        print_fail(f"Error: {str(e)}")
        return False

def main():
    passed = 0
    failed = 0
    
    print_header("TrinityChain Dashboard API Tests")
    print(f"Timestamp: {datetime.now().isoformat()}\n")
    
    # Test 1: API Server Connectivity
    print_header("Test 1: API Server Connectivity")
    print_test("Checking API server on port 3000")
    try:
        response = requests.get(f"{API_URL}/health", timeout=5)
        print_pass(f"API server is running (Status: {response.status_code})")
        passed += 1
    except Exception as e:
        print_fail(f"API server not responding: {str(e)}")
        print("Start with: cargo run --release --bin trinity-api")
        failed += 1
        return
    
    # Test 2: Dashboard Connectivity
    print_header("Test 2: Dashboard Server")
    print_test("Checking dashboard on port 5173")
    try:
        response = requests.get(DASHBOARD_URL, timeout=5)
        print_pass(f"Dashboard is running (Status: {response.status_code})")
        passed += 1
    except Exception as e:
        print_warn(f"Dashboard not responding: {str(e)}")
        print("Start with: cd dashboard && npm run dev")
    
    # Test 3: Blockchain Endpoints
    print_header("Test 3: Blockchain Endpoints")
    
    endpoints = [
        ("/api/blockchain/height", "GET", "Get blockchain height"),
        ("/api/blockchain/blocks", "GET", "Get blocks (limit 10)"),
        ("/api/blockchain/stats", "GET", "Get blockchain stats"),
    ]
    
    for path, method, desc in endpoints:
        if test_endpoint(path, method, desc):
            passed += 1
        else:
            failed += 1
    
    # Test 4: Transaction Endpoints
    print_header("Test 4: Transaction Endpoints")
    
    endpoints = [
        ("/api/transaction", "POST", "Submit transaction"),
        ("/api/mempool", "GET", "Get mempool"),
    ]
    
    for path, method, desc in endpoints:
        if test_endpoint(path, method, desc):
            passed += 1
        else:
            failed += 1
    
    # Test 5: Mining Endpoints
    print_header("Test 5: Mining Endpoints")
    
    endpoints = [
        ("/api/mining/status", "GET", "Get mining status"),
    ]
    
    for path, method, desc in endpoints:
        if test_endpoint(path, method, desc):
            passed += 1
        else:
            failed += 1
    
    # Test 6: Network Endpoints
    print_header("Test 6: Network Endpoints")
    
    endpoints = [
        ("/api/network/peers", "GET", "Get network peers"),
        ("/api/network/info", "GET", "Get network info"),
    ]
    
    for path, method, desc in endpoints:
        if test_endpoint(path, method, desc):
            passed += 1
        else:
            failed += 1
    
    # Test 7: System Endpoints
    print_header("Test 7: System Endpoints")
    
    endpoints = [
        ("/api/health", "GET", "Health check"),
        ("/api/stats", "GET", "API statistics"),
    ]
    
    for path, method, desc in endpoints:
        if test_endpoint(path, method, desc):
            passed += 1
        else:
            failed += 1
    
    # Test 8: Wallet Endpoint
    print_header("Test 8: Wallet Endpoint")
    if test_endpoint("/api/wallet/create", "POST", "Create wallet"):
        passed += 1
    else:
        failed += 1
    
    # Test 9: CORS Configuration
    print_header("Test 9: CORS Configuration")
    print_test("Check CORS headers")
    try:
        response = requests.options(f"{API_URL}/api/blockchain/stats", timeout=5)
        cors_origin = response.headers.get('access-control-allow-origin', 'NOT SET')
        cors_methods = response.headers.get('access-control-allow-methods', 'NOT SET')
        
        if cors_origin and cors_origin != 'NOT SET':
            print_pass(f"CORS enabled: {cors_origin}")
            passed += 1
        else:
            print_warn("CORS headers may not be configured properly")
    except Exception as e:
        print_fail(f"Error checking CORS: {str(e)}")
        failed += 1
    
    # Test 10: API Response Format
    print_header("Test 10: API Response Quality")
    print_test("Fetch blockchain stats")
    try:
        response = requests.get(f"{API_URL}/api/blockchain/stats", timeout=5)
        data = response.json()
        
        required_fields = ['height', 'difficulty', 'mempool_size', 'total_blocks']
        missing = [f for f in required_fields if f not in data]
        
        if not missing:
            print_pass(f"All required fields present")
            print(f"    Data: {json.dumps(data, indent=2)}")
            passed += 1
        else:
            print_fail(f"Missing fields: {missing}")
            print(f"    Got: {json.dumps(data, indent=2)}")
            failed += 1
    except Exception as e:
        print_fail(f"Error: {str(e)}")
        failed += 1
    
    # Summary
    print_header("Test Summary")
    print(f"{GREEN}Passed: {passed}{RESET}")
    print(f"{RED}Failed: {failed}{RESET}")
    print()
    
    if failed == 0:
        print(f"{GREEN}✓ All tests passed!{RESET}")
        print("\nNext steps:")
        print("1. Refresh dashboard: http://localhost:5173")
        print("2. Open browser DevTools (F12)")
        print("3. Go to Console tab to check for errors")
        print("4. Go to Network tab to see API calls")
        print("5. Try creating a wallet in the Wallet tab")
        return 0
    else:
        print(f"{RED}✗ Some tests failed{RESET}")
        print("\nTroubleshooting:")
        print("• Make sure both servers are running:")
        print("  1. cargo run --release --bin trinity-api")
        print("  2. cd dashboard && npm run dev")
        print("• Check that no other processes are using ports 3000 or 5173")
        print("• Check the server logs for errors")
        return 1

if __name__ == "__main__":
    sys.exit(main())
