# GaussMeridian Load Testing Script
# Uses k6 for load testing - https://k6.io/

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

// Custom metrics
const requestDuration = new Trend('request_duration', true);
const successRate = new Rate('success_rate');
const rateLimitCounter = new Counter('rate_limit_errors');

// Test configuration
export const options = {
    stages: [
        { duration: '30s', target: 10 },   // Ramp up to 10 users
        { duration: '1m', target: 50 },    // Ramp up to 50 users
        { duration: '2m', target: 100 },   // Stay at 100 users
        { duration: '1m', target: 200 },   // Spike to 200 users
        { duration: '30s', target: 0 },    // Ramp down
    ],
    thresholds: {
        http_req_duration: ['p(95)<500', 'p(99)<1000'],  // 95% < 500ms, 99% < 1s
        http_req_failed: ['rate<0.1'],                    // Error rate < 10%
        success_rate: ['rate>0.9'],                       // Success rate > 90%
    },
};

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:3000';
const API_KEY = __ENV.API_KEY || 'test-api-key-1234567890abcdef';

// Test setup
export function setup() {
    // Register a test user
    const registerRes = http.post(`${BASE_URL}/v1/auth/register`, JSON.stringify({
        email: `loadtest-${Date.now()}@example.com`,
        username: `loadtest_${Date.now()}`,
        password: 'SecureLoadTestPass123!',
    }), {
        headers: { 'Content-Type': 'application/json' },
    });

    if (registerRes.status === 201) {
        const data = JSON.parse(registerRes.body);
        return {
            token: data.token,
            userId: data.user.id,
        };
    }

    return { token: null, userId: null };
}

// Main test function
export default function (data) {
    const headers = {
        'Content-Type': 'application/json',
        'x-api-key': API_KEY,
    };

    // If we have a JWT token from setup, use it
    if (data.token) {
        headers['Authorization'] = `Bearer ${data.token}`;
    }

    // Test 1: Health check
    testHealthCheck();

    // Test 2: List models
    testListModels(headers);

    // Test 3: Chat completion request
    testChatCompletion(headers);

    // Test 4: Get usage
    if (data.userId) {
        testGetUsage(headers);
    }

    sleep(1); // Think time between iterations
}

function testHealthCheck() {
    const res = http.get(`${BASE_URL}/health`);
    
    const success = check(res, {
        'health check status is 200': (r) => r.status === 200,
        'health check response time < 100ms': (r) => r.timings.duration < 100,
    });
    
    successRate.add(success);
    requestDuration.add(res.timings.duration);
}

function testListModels(headers) {
    const res = http.get(`${BASE_URL}/v1/models`, { headers });
    
    const success = check(res, {
        'list models status is 200': (r) => r.status === 200,
        'list models has data': (r) => {
            try {
                const data = JSON.parse(r.body);
                return data.data && Array.isArray(data.data);
            } catch {
                return false;
            }
        },
    });
    
    successRate.add(success);
    requestDuration.add(res.timings.duration);
}

function testChatCompletion(headers) {
    const payload = JSON.stringify({
        model: 'gpt-3.5-turbo',
        messages: [
            { role: 'system', content: 'You are a helpful assistant.' },
            { role: 'user', content: 'Say hello!' },
        ],
        max_tokens: 50,
    });

    const res = http.post(`${BASE_URL}/v1/chat/completions`, payload, { headers });
    
    const success = check(res, {
        'chat completion status is 200 or 429': (r) => r.status === 200 || r.status === 429,
        'chat completion response time < 2000ms': (r) => r.timings.duration < 2000,
    });
    
    if (res.status === 429) {
        rateLimitCounter.add(1);
    }
    
    successRate.add(success);
    requestDuration.add(res.timings.duration);
}

function testGetUsage(headers) {
    const res = http.get(`${BASE_URL}/v1/balance`, { headers });
    
    const success = check(res, {
        'get balance status is 200': (r) => r.status === 200,
    });
    
    successRate.add(success);
    requestDuration.add(res.timings.duration);
}

// Teardown
export function teardown(data) {
    // Clean up test data if needed
    console.log('Load test completed');
}

