gh repo list --limit 1000 \
  --json name,description,url,homepageUrl,primaryLanguage,repositoryTopics,stargazerCount,isPrivate,isFork,createdAt,updatedAt \
  | jq -r '
    ["name","description","url","homepage","language","topics","stars","private","fork","created_at","updated_at"],
    (.[] | [
      .name,
      .description,
      .url,
      .homepageUrl,
      .primaryLanguage.name,
      ((.repositoryTopics // []) | map(.name) | join(";")),
      .stargazerCount,
      .isPrivate,
      .isFork,
      .createdAt,
      .updatedAt
    ])
    | @csv' > repos.csv