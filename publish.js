import { Octokit } from "@octokit/rest";
import fs from "fs";
import path from "path";

const octokit = new Octokit({ auth: process.env.GITHUB_TOKEN });

async function uploadFile() {
    try {
        const filePath = path.join("docs", "index.html");
        const content = fs.readFileSync(filePath, { encoding: "base64" });
        const date = new Date().toISOString().split("T")[0];

        await octokit.repos.createOrUpdateFileContents({
            owner: "BuildersSC",
            repo: "Krevetka",
            path: "docs/index.html",
            message: `Update ChangeLog on ${date}`,
            content: content,
            branch: "gh-pages",
        });

        console.log("File uploaded successfully!");
    } catch (err) {
        console.error("Upload failed:", err);
        process.exit(1);
    }
}

uploadFile();