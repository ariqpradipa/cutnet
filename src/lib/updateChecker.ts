interface GitHubRelease {
  tag_name: string;
  name: string;
  body: string;
  published_at: string;
  html_url: string;
}

interface UpdateCheckResult {
  available: boolean;
  version?: string;
  currentVersion: string;
  releaseNotes?: string;
  downloadUrl?: string;
  error?: string;
}

const CURRENT_VERSION = "0.1.0";
const GITHUB_API_URL = "https://api.github.com/repos/encore/cutnet/releases/latest";
const GITHUB_RELEASES_URL = "https://github.com/encore/cutnet/releases";

function parseVersion(version: string): number[] {
  return version
    .replace(/^v/, "")
    .split(".")
    .map((n) => parseInt(n, 10) || 0);
}

function compareVersions(v1: string, v2: string): number {
  const parts1 = parseVersion(v1);
  const parts2 = parseVersion(v2);

  for (let i = 0; i < Math.max(parts1.length, parts2.length); i++) {
    const p1 = parts1[i] || 0;
    const p2 = parts2[i] || 0;
    if (p1 !== p2) {
      return p1 - p2;
    }
  }
  return 0;
}

export async function checkForUpdates(): Promise<UpdateCheckResult> {
  try {
    const response = await fetch(GITHUB_API_URL, {
      headers: {
        Accept: "application/vnd.github.v3+json",
        "User-Agent": "CutNet-UpdateChecker/1.0",
      },
    });

    if (!response.ok) {
      if (response.status === 404) {
        return {
          available: false,
          currentVersion: CURRENT_VERSION,
          error: "No releases found",
        };
      }
      throw new Error(`GitHub API error: ${response.status}`);
    }

    const release: GitHubRelease = await response.json();
    const latestVersion = release.tag_name;

    const comparison = compareVersions(latestVersion, CURRENT_VERSION);

    if (comparison > 0) {
      return {
        available: true,
        version: latestVersion,
        currentVersion: CURRENT_VERSION,
        releaseNotes: release.body,
        downloadUrl: release.html_url,
      };
    }

    return {
      available: false,
      currentVersion: CURRENT_VERSION,
    };
  } catch (error) {
    console.error("Failed to check for updates:", error);
    return {
      available: false,
      currentVersion: CURRENT_VERSION,
      error: error instanceof Error ? error.message : "Unknown error",
    };
  }
}

export function getReleasesUrl(): string {
  return GITHUB_RELEASES_URL;
}

export function getCurrentVersion(): string {
  return CURRENT_VERSION;
}
