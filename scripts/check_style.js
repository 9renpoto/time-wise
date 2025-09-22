#!/usr/bin/env node
const fs = require("fs");
const path = require("path");

function checkJson(filePath, content) {
  try {
    const parsed = JSON.parse(content);
    const formatted = JSON.stringify(parsed, null, 2) + "\n";
    if (content !== formatted) {
      return `JSON formatting mismatch: ${filePath}`;
    }
  } catch (err) {
    return `Invalid JSON (${filePath}): ${err.message}`;
  }
  return null;
}

function checkCss(filePath, content) {
  const issues = [];
  const lines = content.split(/\r?\n/);
  lines.forEach((line, index) => {
    if (/\s+$/.test(line)) {
      issues.push(`Trailing whitespace: ${filePath}:${index + 1}`);
    }
    if (line.includes("\t")) {
      issues.push(`Tab character found: ${filePath}:${index + 1}`);
    }
  });
  if (!content.endsWith("\n")) {
    issues.push(`File must end with newline: ${filePath}`);
  }
  return issues.length > 0 ? issues : null;
}

const files = process.argv.slice(2);
if (files.length === 0) {
  process.exit(0);
}

const problems = [];
files.forEach((filePath) => {
  let content;
  try {
    content = fs.readFileSync(filePath, "utf8");
  } catch (err) {
    // Ignore unreadable files (likely removed)
    return;
  }
  const ext = path.extname(filePath).toLowerCase();
  if (ext === ".json") {
    const issue = checkJson(filePath, content);
    if (issue) {
      problems.push(issue);
    }
  } else if (ext === ".css") {
    const issueList = checkCss(filePath, content);
    if (issueList) {
      problems.push(...issueList);
    }
  }
});

if (problems.length > 0) {
  console.error(problems.join("\n"));
  process.exit(1);
}
