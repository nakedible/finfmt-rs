module.exports = {
  extends: ["@commitlint/config-conventional"],
  rules: {
    "subject-case": [0],
    "type-enum": [
      2,
      "always",
      ["changed", "chore", "deprecated", "feat", "fix", "removed", "security"],
    ],
  },
};
