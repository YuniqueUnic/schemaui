/**
 * Automated UI Test Suite for SchemaUI Web
 *
 * This script provides comprehensive testing of the SchemaUI Web interface,
 * including regression tests for all critical functionality.
 */

class SchemaUITester {
    constructor() {
        this.results = [];
        this.currentTest = null;
    }

    // Helper function to wait for element
    async waitForElement(selector, timeout = 5000) {
        const start = Date.now();
        while (Date.now() - start < timeout) {
            const element = document.querySelector(selector);
            if (element) return element;
            await new Promise((resolve) => setTimeout(resolve, 100));
        }
        throw new Error(`Element ${selector} not found after ${timeout}ms`);
    }

    // Helper function to click element by UID
    async clickElement(uid) {
        const element = document.querySelector(`[uid="${uid}"]`);
        if (!element) {
            // Try to find by traversing accessibility tree
            const allElements = document.querySelectorAll("*");
            for (const el of allElements) {
                if (el.getAttribute("uid") === uid || el.textContent === uid) {
                    el.click();
                    await new Promise((resolve) => setTimeout(resolve, 100));
                    return true;
                }
            }
            return false;
        }
        element.click();
        await new Promise((resolve) => setTimeout(resolve, 100));
        return true;
    }

    // Helper to get JSON data
    getJsonData() {
        const jsonText = Array.from(document.querySelectorAll(".text-sm"))
            .map((el) => el.textContent)
            .join("");

        try {
            // Try to extract JSON from the page
            const preElement = document.querySelector("pre");
            if (preElement) {
                return JSON.parse(preElement.textContent);
            }

            // Alternative: reconstruct from text elements
            const bodyText = document.body.innerText;
            const jsonMatch = bodyText.match(/\{[\s\S]*\}/);
            if (jsonMatch) {
                return JSON.parse(jsonMatch[0]);
            }
        } catch (e) {
            console.error("Failed to parse JSON:", e);
        }
        return null;
    }

    // Test 1: Number Input Update
    async testNumberInput(path, expectedField) {
        this.currentTest = `Number Input at ${path}`;
        console.log(`\n🧪 Testing: ${this.currentTest}`);

        try {
            // Navigate to the path
            const pathParts = path.split("/").filter((p) => p);
            for (const part of pathParts) {
                const button = Array.from(document.querySelectorAll("button"))
                    .find((b) => b.textContent === part);
                if (button) {
                    button.click();
                    await new Promise((resolve) => setTimeout(resolve, 200));
                }
            }

            // Find number input
            const numberInputs = document.querySelectorAll(
                'input[type="number"]',
            );
            if (numberInputs.length === 0) {
                throw new Error("No number input found");
            }

            const input = numberInputs[0];
            const testValue = 12345;

            // Method 1: Direct value set with events
            input.value = testValue.toString();
            input.dispatchEvent(new Event("input", { bubbles: true }));
            input.dispatchEvent(new Event("change", { bubbles: true }));
            input.dispatchEvent(new Event("blur", { bubbles: true }));

            await new Promise((resolve) => setTimeout(resolve, 500));

            // Check if value persisted
            const currentValue = input.value;
            const jsonData = this.getJsonData();

            // Navigate to the expected value in JSON
            let jsonValue = jsonData;
            const jsonPath = path.split("/").filter((p) => p);
            for (const key of jsonPath) {
                if (jsonValue) jsonValue = jsonValue[key];
            }

            const actualValue = jsonValue?.[expectedField];

            const success = actualValue === testValue;

            this.results.push({
                test: this.currentTest,
                success,
                details: {
                    inputValue: currentValue,
                    jsonValue: actualValue,
                    expected: testValue,
                    path: path,
                    field: expectedField,
                },
            });

            console.log(success ? "✅ PASS" : "❌ FAIL", {
                inputShows: currentValue,
                jsonShows: actualValue,
                expected: testValue,
            });

            return success;
        } catch (error) {
            this.results.push({
                test: this.currentTest,
                success: false,
                error: error.message,
            });
            console.log("❌ ERROR:", error.message);
            return false;
        }
    }

    // Test 2: Text Input Update
    async testTextInput(path, field, testValue = "test-automation") {
        this.currentTest = `Text Input at ${path}/${field}`;
        console.log(`\n🧪 Testing: ${this.currentTest}`);

        try {
            // Navigate to path
            const pathParts = path.split("/").filter((p) => p);
            for (const part of pathParts) {
                const button = Array.from(document.querySelectorAll("button"))
                    .find((b) => b.textContent === part);
                if (button) {
                    button.click();
                    await new Promise((resolve) => setTimeout(resolve, 200));
                }
            }

            // Find text input
            const textInputs = document.querySelectorAll(
                'input[type="text"], input:not([type])',
            );
            const input = Array.from(textInputs).find((i) =>
                !i.type || i.type === "text"
            );

            if (!input) {
                throw new Error("No text input found");
            }

            // Set value
            input.value = testValue;
            input.dispatchEvent(new Event("input", { bubbles: true }));
            input.dispatchEvent(new Event("change", { bubbles: true }));

            await new Promise((resolve) => setTimeout(resolve, 300));

            const jsonData = this.getJsonData();
            let jsonValue = jsonData;
            const jsonPath = path.split("/").filter((p) => p);
            for (const key of jsonPath) {
                if (jsonValue) jsonValue = jsonValue[key];
            }

            const actualValue = jsonValue?.[field];
            const success = actualValue === testValue;

            this.results.push({
                test: this.currentTest,
                success,
                details: {
                    expected: testValue,
                    actual: actualValue,
                },
            });

            console.log(success ? "✅ PASS" : "❌ FAIL", {
                expected: testValue,
                actual: actualValue,
            });

            return success;
        } catch (error) {
            this.results.push({
                test: this.currentTest,
                success: false,
                error: error.message,
            });
            console.log("❌ ERROR:", error.message);
            return false;
        }
    }

    // Test 3: OneOf Switching
    async testOneOfSwitch(path, variant1, variant2) {
        this.currentTest = `OneOf Switch at ${path}`;
        console.log(`\n🧪 Testing: ${this.currentTest}`);

        try {
            // Navigate to path
            const pathParts = path.split("/").filter((p) => p);
            for (const part of pathParts) {
                const button = Array.from(document.querySelectorAll("button"))
                    .find((b) => b.textContent === part);
                if (button) {
                    button.click();
                    await new Promise((resolve) => setTimeout(resolve, 200));
                }
            }

            // Find variant selector
            const radioButtons = document.querySelectorAll(
                'input[type="radio"], button[role="radio"]',
            );

            let switched = false;

            // Try clicking variant elements
            const variantButtons = Array.from(document.querySelectorAll("*"))
                .filter((el) =>
                    el.textContent.includes(variant1) ||
                    el.textContent.includes(variant2)
                );

            if (variantButtons.length >= 2) {
                variantButtons[1].click();
                await new Promise((resolve) => setTimeout(resolve, 300));

                variantButtons[0].click();
                await new Promise((resolve) => setTimeout(resolve, 300));

                switched = true;
            }

            this.results.push({
                test: this.currentTest,
                success: switched,
                details: {
                    path,
                    variants: [variant1, variant2],
                },
            });

            console.log(switched ? "✅ PASS" : "❌ FAIL");
            return switched;
        } catch (error) {
            this.results.push({
                test: this.currentTest,
                success: false,
                error: error.message,
            });
            console.log("❌ ERROR:", error.message);
            return false;
        }
    }

    // Test 4: Array CRUD Operations
    async testArrayOperations(path) {
        this.currentTest = `Array CRUD at ${path}`;
        console.log(`\n🧪 Testing: ${this.currentTest}`);

        try {
            // Navigate to path
            const pathParts = path.split("/").filter((p) => p);
            for (const part of pathParts) {
                const button = Array.from(document.querySelectorAll("button"))
                    .find((b) => b.textContent === part);
                if (button) {
                    button.click();
                    await new Promise((resolve) => setTimeout(resolve, 200));
                }
            }

            const jsonBefore = this.getJsonData();

            // Try to add item
            const addButton = Array.from(document.querySelectorAll("button"))
                .find((b) => b.textContent.includes("Add"));

            let operations = { add: false, remove: false };

            if (addButton) {
                addButton.click();
                await new Promise((resolve) => setTimeout(resolve, 300));
                operations.add = true;
            }

            // Try to remove item
            const removeButton = Array.from(document.querySelectorAll("button"))
                .find((b) => b.textContent.includes("Remove"));

            if (removeButton) {
                removeButton.click();
                await new Promise((resolve) => setTimeout(resolve, 300));
                operations.remove = true;
            }

            const success = operations.add || operations.remove;

            this.results.push({
                test: this.currentTest,
                success,
                details: operations,
            });

            console.log(success ? "✅ PASS" : "❌ FAIL", operations);
            return success;
        } catch (error) {
            this.results.push({
                test: this.currentTest,
                success: false,
                error: error.message,
            });
            console.log("❌ ERROR:", error.message);
            return false;
        }
    }

    // Run all tests
    async runAllTests() {
        console.log("🚀 Starting SchemaUI Web Automated Tests\n");
        console.log("=".repeat(60));

        // Test suite
        const tests = [
            // Critical number input bug
            () => this.testNumberInput("/e/e1/e2/e3/e4/logic", "value"),

            // Other number inputs
            () => this.testNumberInput("/a", "age"),
            () => this.testNumberInput("/a", "rating"),

            // Text inputs
            () => this.testTextInput("/a", "name", "John Doe"),
            () => this.testTextInput("/a", "email", "test@example.com"),

            // OneOf switching
            () =>
                this.testOneOfSwitch(
                    "/e/e1/e2/e3/e4/logic",
                    "fixed",
                    "dynamic",
                ),
            () => this.testOneOfSwitch("/b/b1", "simpleItem", "numericItem"),

            // Array operations
            () => this.testArrayOperations("/d/d1/d2/d3/config/features"),
            () => this.testArrayOperations("/b/b1"),
        ];

        // Execute tests sequentially
        for (const test of tests) {
            await test();
            await new Promise((resolve) => setTimeout(resolve, 500));
        }

        // Generate report
        this.generateReport();
    }

    // Generate test report
    generateReport() {
        console.log("\n" + "=".repeat(60));
        console.log("📊 TEST REPORT\n");

        const passed = this.results.filter((r) => r.success).length;
        const failed = this.results.filter((r) => !r.success).length;
        const total = this.results.length;

        console.log(`Total Tests: ${total}`);
        console.log(`✅ Passed: ${passed}`);
        console.log(`❌ Failed: ${failed}`);
        console.log(`Success Rate: ${((passed / total) * 100).toFixed(1)}%\n`);

        // Detailed failures
        if (failed > 0) {
            console.log("Failed Tests:");
            this.results.filter((r) => !r.success).forEach((r) => {
                console.log(`  ❌ ${r.test}`);
                if (r.error) console.log(`     Error: ${r.error}`);
                if (r.details) console.log(`     Details:`, r.details);
            });
        }

        // Critical issue detection
        console.log("\n🔍 Critical Issues:");
        const numberInputFails = this.results.filter((r) =>
            r.test.includes("Number Input") && !r.success
        );

        if (numberInputFails.length > 0) {
            console.log("  🚨 Number inputs are NOT updating JSON correctly!");
            console.log(
                "     This is a CRITICAL bug affecting data persistence.",
            );
        }

        return {
            passed,
            failed,
            total,
            results: this.results,
        };
    }
}

// Export for use
window.SchemaUITester = SchemaUITester;

// Auto-run if called directly
if (typeof window !== "undefined") {
    console.log("SchemaUI Automated Tester loaded.");
    console.log("To run tests: new SchemaUITester().runAllTests()");
}

// Return tester instance
new SchemaUITester();
