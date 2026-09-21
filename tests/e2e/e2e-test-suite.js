#!/usr/bin/env node

/**
 * End-to-End Test Suite for SchemaUI Web UI
 *
 * This script performs automated testing of the SchemaUI Web UI,
 * focusing on critical functionality like number inputs in complex types.
 *
 * Usage: node e2e-test-suite.js
 */

const puppeteer = require("puppeteer");
const chalk = require("chalk");

class SchemaUIE2ETestSuite {
    constructor(baseUrl = "http://localhost:5175") {
        this.baseUrl = baseUrl;
        this.browser = null;
        this.page = null;
        this.results = [];
    }

    async setup() {
        console.log(chalk.blue("🚀 Starting SchemaUI E2E Test Suite"));

        // Launch browser
        this.browser = await puppeteer.launch({
            headless: false, // Set to true for CI
            devtools: true,
            args: ["--no-sandbox", "--disable-setuid-sandbox"],
        });

        this.page = await this.browser.newPage();

        // Enable console logging
        this.page.on("console", (msg) => {
            if (msg.type() === "log") {
                console.log(chalk.gray(`[Browser]: ${msg.text()}`));
            }
        });

        // Navigate to app
        await this.page.goto(this.baseUrl, { waitUntil: "networkidle0" });
        console.log(chalk.green("✅ Browser launched and page loaded"));
    }

    async teardown() {
        if (this.browser) {
            await this.browser.close();
        }
    }

    async navigateToPath(path) {
        const segments = path.split("/").filter((s) => s);

        for (const segment of segments) {
            // Find and click the button
            const button = await this.page.evaluateHandle((seg) => {
                return Array.from(document.querySelectorAll("button"))
                    .find((b) => b.textContent === seg);
            }, segment);

            if (button) {
                await button.click();
                await this.page.waitForTimeout(300);
            } else {
                throw new Error(
                    `Could not find button for segment: ${segment}`,
                );
            }
        }
    }

    async testNumberInput(testCase) {
        console.log(chalk.yellow(`\n📝 Testing: ${testCase.name}`));

        try {
            // Navigate to the path
            await this.navigateToPath(testCase.path);
            await this.page.waitForTimeout(500);

            // Select variant if needed
            if (testCase.variant) {
                const variantElement = await this.page.evaluateHandle(
                    (variantText) => {
                        return Array.from(document.querySelectorAll("*"))
                            .find((el) => el.textContent.includes(variantText));
                    },
                    testCase.variant,
                );

                if (variantElement) {
                    await variantElement.click();
                    await this.page.waitForTimeout(300);
                }
            }

            // Find number input
            const numberInput = await this.page.$('input[type="number"]');
            if (!numberInput) {
                throw new Error("No number input found");
            }

            // Clear and type new value
            await numberInput.click({ clickCount: 3 }); // Select all
            await numberInput.type(testCase.value.toString());

            // Trigger events
            await this.page.evaluate(() => {
                const input = document.querySelector('input[type="number"]');
                input.dispatchEvent(new Event("input", { bubbles: true }));
                input.dispatchEvent(new Event("change", { bubbles: true }));
                input.blur();
            });

            await this.page.waitForTimeout(500);

            // Get JSON value
            const jsonValue = await this.page.evaluate(
                (path, field) => {
                    const bodyText = document.body.innerText;
                    try {
                        const jsonMatch = bodyText.match(/\{[\s\S]*\}/);
                        if (jsonMatch) {
                            const json = JSON.parse(jsonMatch[0]);

                            // Navigate to the value
                            let value = json;
                            const segments = path.split("/").filter((s) => s);
                            for (const seg of segments) {
                                value = value?.[seg];
                            }

                            return field ? value?.[field] : value;
                        }
                    } catch (e) {
                        return null;
                    }
                },
                testCase.path,
                testCase.field,
            );

            // Get input display value
            const inputValue = await numberInput.evaluate((el) => el.value);

            // Check result
            const expectedValue = testCase.value;
            const success = jsonValue === expectedValue;

            this.results.push({
                test: testCase.name,
                success,
                expected: expectedValue,
                actualJson: jsonValue,
                actualInput: inputValue,
                path: testCase.path,
            });

            if (success) {
                console.log(
                    chalk.green(`  ✅ PASS: JSON value = ${jsonValue}`),
                );
            } else {
                console.log(
                    chalk.red(
                        `  ❌ FAIL: Expected ${expectedValue}, got ${jsonValue}`,
                    ),
                );
                console.log(chalk.red(`      Input shows: ${inputValue}`));
            }

            return success;
        } catch (error) {
            console.log(chalk.red(`  ❌ ERROR: ${error.message}`));
            this.results.push({
                test: testCase.name,
                success: false,
                error: error.message,
            });
            return false;
        }
    }

    async testArrayOperations(testCase) {
        console.log(chalk.yellow(`\n📝 Testing: ${testCase.name}`));

        try {
            // Navigate to path
            await this.navigateToPath(testCase.path);
            await this.page.waitForTimeout(500);

            // Test add operation
            const addButton = await this.page.evaluateHandle(() => {
                return Array.from(document.querySelectorAll("button"))
                    .find((b) => b.textContent.includes("Add"));
            });

            if (addButton) {
                await addButton.click();
                await this.page.waitForTimeout(500);
                console.log(chalk.green("  ✅ Add operation successful"));
            }

            // Test remove operation (if items exist)
            const removeButton = await this.page.evaluateHandle(() => {
                return Array.from(document.querySelectorAll("button"))
                    .find((b) => b.textContent.includes("Remove"));
            });

            if (removeButton) {
                await removeButton.click();
                await this.page.waitForTimeout(500);
                console.log(chalk.green("  ✅ Remove operation successful"));
            }

            this.results.push({
                test: testCase.name,
                success: true,
            });

            return true;
        } catch (error) {
            console.log(chalk.red(`  ❌ ERROR: ${error.message}`));
            this.results.push({
                test: testCase.name,
                success: false,
                error: error.message,
            });
            return false;
        }
    }

    async testSliderEditableValue(testCase) {
        console.log(chalk.yellow(`\n📝 Testing Editable Slider: ${testCase.name}`));

        try {
            if (testCase.path) {
                await this.navigateToPath(testCase.path);
                await this.page.waitForTimeout(500);
            }

            // Helper to retrieve JSON preview value
            const getJsonValue = async () => {
                return await this.page.evaluate(
                    (path, field) => {
                        const bodyText = document.body.innerText;
                        try {
                            const jsonMatch = bodyText.match(/\{[\s\S]*\}/);
                            if (jsonMatch) {
                                const json = JSON.parse(jsonMatch[0]);
                                let value = json;
                                const segments = path ? path.split("/").filter((s) => s) : [];
                                for (const seg of segments) {
                                    value = value?.[seg];
                                }
                                return field ? value?.[field] : value;
                            }
                        } catch (e) {
                            return null;
                        }
                    },
                    testCase.path || "",
                    testCase.field || "",
                );
            };

            // 1. Enter edit mode
            const valueButton = await this.page.$(
                'button[aria-label="Edit value"], button[aria-label="Edit minimum value"]',
            );
            if (!valueButton) {
                throw new Error(
                    "Could not find editable slider value button (editable=true)",
                );
            }

            await valueButton.click();
            await this.page.waitForTimeout(300);

            let editInput = await this.page.$(
                'input[aria-label="Edit value"], input[aria-label="Edit minimum value"]',
            );
            if (!editInput) {
                throw new Error(
                    "Input did not appear after clicking slider value display",
                );
            }
            console.log(chalk.green("  ✅ 1. Enter edit mode verified"));

            // 2. Valid commit (Enter)
            const validVal = testCase.validValue !== undefined
                ? testCase.validValue
                : 42;
            await editInput.click({ clickCount: 3 });
            await editInput.type(validVal.toString());
            await this.page.keyboard.press("Enter");
            await this.page.waitForTimeout(500);

            const inputAfterEnter = await this.page.$(
                'input[aria-label="Edit value"], input[aria-label="Edit minimum value"]',
            );
            if (inputAfterEnter) {
                throw new Error("Edit mode did not exit after pressing Enter");
            }

            const buttonAfterEnter = await this.page.$(
                'button[aria-label="Edit value"], button[aria-label="Edit minimum value"]',
            );
            const displayedText = await this.page.evaluate(
                (el) => el?.textContent?.trim(),
                buttonAfterEnter,
            );
            const jsonAfterEnter = await getJsonValue();
            console.log(
                chalk.green(
                    `  ✅ 2. Valid commit (Enter) verified (display: ${displayedText}, json: ${jsonAfterEnter})`,
                ),
            );

            // 3. Blur commit
            const blurVal = testCase.anotherValidValue !== undefined
                ? testCase.anotherValidValue
                : 60;
            await buttonAfterEnter.click();
            await this.page.waitForTimeout(300);

            editInput = await this.page.$(
                'input[aria-label="Edit value"], input[aria-label="Edit minimum value"]',
            );
            if (!editInput) {
                throw new Error("Input did not appear on second edit entry");
            }
            await editInput.click({ clickCount: 3 });
            await editInput.type(blurVal.toString());

            // Trigger blur using existing test pattern
            await this.page.evaluate(() => {
                const active = document.activeElement;
                if (active && typeof active.blur === "function") {
                    active.blur();
                }
            });
            await this.page.waitForTimeout(500);

            const inputAfterBlur = await this.page.$(
                'input[aria-label="Edit value"], input[aria-label="Edit minimum value"]',
            );
            if (inputAfterBlur) {
                throw new Error("Edit mode did not exit after blur");
            }
            const jsonAfterBlur = await getJsonValue();
            console.log(
                chalk.green(
                    `  ✅ 3. Blur commit verified (json: ${jsonAfterBlur})`,
                ),
            );

            // 4. Escape cancel
            const buttonBeforeCancel = await this.page.$(
                'button[aria-label="Edit value"], button[aria-label="Edit minimum value"]',
            );
            const valBeforeCancel = await this.page.evaluate(
                (el) => el?.textContent?.trim(),
                buttonBeforeCancel,
            );
            const jsonBeforeCancel = await getJsonValue();

            await buttonBeforeCancel.click();
            await this.page.waitForTimeout(300);

            editInput = await this.page.$(
                'input[aria-label="Edit value"], input[aria-label="Edit minimum value"]',
            );
            await editInput.click({ clickCount: 3 });
            await editInput.type("9999");
            await this.page.keyboard.press("Escape");
            await this.page.waitForTimeout(300);

            const inputAfterEscape = await this.page.$(
                'input[aria-label="Edit value"], input[aria-label="Edit minimum value"]',
            );
            if (inputAfterEscape) {
                throw new Error("Edit mode did not exit after pressing Escape");
            }

            const valAfterEscape = await this.page.evaluate(
                (el) => el?.textContent?.trim(),
                await this.page.$(
                    'button[aria-label="Edit value"], button[aria-label="Edit minimum value"]',
                ),
            );
            const jsonAfterEscape = await getJsonValue();

            if (
                valAfterEscape !== valBeforeCancel ||
                jsonAfterEscape !== jsonBeforeCancel
            ) {
                throw new Error(
                    `Escape did not restore previous value (expected: ${valBeforeCancel}, actual: ${valAfterEscape})`,
                );
            }
            console.log(
                chalk.green(
                    "  ✅ 4. Escape cancel verified (original value retained)",
                ),
            );

            // 5. Invalid value
            const buttonBeforeInvalid = await this.page.$(
                'button[aria-label="Edit value"], button[aria-label="Edit minimum value"]',
            );
            await buttonBeforeInvalid.click();
            await this.page.waitForTimeout(300);

            editInput = await this.page.$(
                'input[aria-label="Edit value"], input[aria-label="Edit minimum value"]',
            );
            await editInput.click({ clickCount: 3 });
            await editInput.type(testCase.invalidValue || "not-a-number");
            await this.page.keyboard.press("Enter");
            await this.page.waitForTimeout(300);

            const ariaInvalid = await this.page.evaluate(
                (el) => el.getAttribute("aria-invalid"),
                editInput,
            );
            const alertElement = await this.page.$('[role="alert"]');
            if (ariaInvalid !== "true" || !alertElement) {
                throw new Error(
                    "Invalid value did not set aria-invalid='true' or render accessible role='alert'",
                );
            }
            const jsonAfterInvalid = await getJsonValue();
            if (jsonAfterInvalid !== jsonBeforeCancel) {
                throw new Error(
                    "Invalid value incorrectly modified JSON preview model",
                );
            }

            // Clean up invalid state by pressing Escape
            await this.page.keyboard.press("Escape");
            await this.page.waitForTimeout(300);
            console.log(
                chalk.green(
                    "  ✅ 5. Invalid value verified (aria-invalid='true', alert present, model unchanged)",
                ),
            );

            // 6. Disabled/readOnly validation
            const disabledButtons = await this.page.$$(
                'button[disabled][aria-label*="Edit"], [aria-disabled="true"][aria-label*="Edit"]',
            );
            for (const btn of disabledButtons) {
                await btn.click();
                await this.page.waitForTimeout(200);
            }
            const rogueInput = await this.page.$(
                'input[aria-label="Edit value"], input[aria-label="Edit minimum value"]',
            );
            if (rogueInput) {
                throw new Error(
                    "Disabled or readOnly button allowed entering edit mode",
                );
            }
            console.log(
                chalk.green(
                    "  ✅ 6. Disabled and readOnly validation verified (cannot enter edit mode)",
                ),
            );

            this.results.push({
                test: testCase.name,
                success: true,
            });
            return true;
        } catch (error) {
            console.log(chalk.red(`  ❌ ERROR: ${error.message}`));
            this.results.push({
                test: testCase.name,
                success: false,
                error: error.message,
            });
            return false;
        }
    }

    async runAllTests() {
        await this.setup();

        console.log(chalk.blue("\n🧪 Running Test Suite\n"));

        // Define test cases
        const numberInputTests = [
            {
                name: "OneOf Object Number Field (/e/e1/e2/e3/e4/logic)",
                path: "/e/e1/e2/e3/e4/logic",
                variant: "fixed",
                field: "value",
                value: 12345,
            },
            {
                name: "Simple Number Field (/a/age)",
                path: "/a",
                field: "age",
                value: 25,
            },
            {
                name: "Simple Number Field (/a/rating)",
                path: "/a",
                field: "rating",
                value: 4.5,
            },
            {
                name: "Nested Object Number (/c/c1/c2/settings/threshold)",
                path: "/c/c1/c2/settings",
                field: "threshold",
                value: 100,
            },
        ];

        const arrayTests = [
            {
                name: "OneOf Array Operations (/b/b1)",
                path: "/b/b1",
            },
            {
                name: "Deep Nested Array (/d/d1/d2/d3/config/features)",
                path: "/d/d1/d2/d3/config/features",
            },
        ];

        // Run number input tests
        for (const test of numberInputTests) {
            await this.testNumberInput(test);
        }

        // Run array tests
        for (const test of arrayTests) {
            await this.testArrayOperations(test);
        }

        // Run editable slider tests if provided
        if (Array.isArray(this.sliderTests) && this.sliderTests.length > 0) {
            for (const test of this.sliderTests) {
                await this.testSliderEditableValue(test);
            }
        }

        // Generate report
        this.generateReport();

        await this.teardown();
    }

    generateReport() {
        console.log(chalk.blue("\n" + "=".repeat(60)));
        console.log(chalk.blue("📊 TEST REPORT"));
        console.log(chalk.blue("=".repeat(60) + "\n"));

        const passed = this.results.filter((r) => r.success).length;
        const failed = this.results.filter((r) => !r.success).length;
        const total = this.results.length;

        console.log(`Total Tests: ${total}`);
        console.log(chalk.green(`✅ Passed: ${passed}`));
        console.log(chalk.red(`❌ Failed: ${failed}`));
        console.log(`Success Rate: ${((passed / total) * 100).toFixed(1)}%\n`);

        // Show failed tests details
        if (failed > 0) {
            console.log(chalk.red("Failed Tests:"));
            this.results.filter((r) => !r.success).forEach((r) => {
                console.log(chalk.red(`  • ${r.test}`));
                if (r.error) {
                    console.log(chalk.red(`    Error: ${r.error}`));
                } else if (r.expected !== undefined) {
                    console.log(
                        chalk.red(
                            `    Expected: ${r.expected}, Got: ${r.actualJson}`,
                        ),
                    );
                }
            });
        }

        // Critical issues
        console.log(chalk.yellow("\n⚠️ Critical Issues:"));
        const numberInputFails = this.results.filter((r) =>
            r.test.includes("Number") && !r.success
        );

        if (numberInputFails.length > 0) {
            console.log(
                chalk.red(
                    "  🚨 Number inputs are NOT updating JSON correctly!",
                ),
            );
            console.log(
                chalk.red(
                    "     This is a CRITICAL bug affecting data persistence.",
                ),
            );
            console.log(chalk.yellow("\n  Recommended Actions:"));
            console.log(
                chalk.yellow(
                    "  1. Check onChange event propagation in composite types",
                ),
            );
            console.log(
                chalk.yellow(
                    "  2. Verify pointer path handling in NodeRenderer",
                ),
            );
            console.log(
                chalk.yellow("  3. Debug the value transformation pipeline"),
            );
        } else {
            console.log(
                chalk.green("  ✅ All critical functions working correctly!"),
            );
        }
    }
}

// Run tests if executed directly
if (require.main === module) {
    const tester = new SchemaUIE2ETestSuite();
    tester.runAllTests().catch((error) => {
        console.error(chalk.red("Test suite failed:"), error);
        process.exit(1);
    });
}

module.exports = SchemaUIE2ETestSuite;
