# Contributing to tonlib-rs

Thank you for considering contributing to tonlib-rs! This project thrives on community collaboration, and we welcome your ideas, bug reports, and improvements. Follow these guidelines to make your contributions smooth and effective.


## How Can You Contribute?

### 1. Reporting Bugs
If you encounter any issues, check the [existing issues](https://github.com/ston-fi/tonlib-rs/issues) first to avoid duplicates. If none match your bug, create a new issue with:
  
* A clear, descriptive title.
* Steps to reproduce the bug.
* Expected and actual results.
* Your environment details (e.g., OS, Rust version, etc.).


### 2. Code Contributions
Contributions to the codebase are highly appreciated. Here's how to start:
    
- Start by forking the main repository to your GitHub account. You can do this via the "Fork" button on the repository page.

- Clone your forked repository locally:


- Create a Feature Branch:

- Follow the coding standards mentioned below while making changes. 
- Add or update tests to cover your changes.
- Ensure your changes are thoroughly tested.

- Before creating a PR, run the test suite to ensure everything works as expected:

> **__NOTE__**: that some tests in ```tonlib-client``` are dependent on the current load of TON blockchain and may be flacky if the load is relatively high. 
It is recommended to run tests with  ```cargo nextest run --retries=10```  

- Create a Pull Request (PR). Provide a brief description of the changes made.


### 3. Feature Requests

Do you have an idea for a new feature? Open an issue and clearly explain:
* The problem it solves.
* How it improves the project.
* Any implementation suggestions.


### 4 Coding standards

- Rust Style: Follow the Rust API Guidelines.
- Formatting: Use cargo +nightly fmt to format your code before submitting.
- Linting: Run cargo clippy and address warnings to maintain code quality.

# Conclusion

If you have any questions, feel free to:
- Open a discussion.
- Contact the maintainers by tagging them in an issue.

We appreciate your time and effort in making tonlib-rs better! 🚀