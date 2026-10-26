import { describe, expect, it } from "vitest";
import { parseSourceChange } from "@/lib/exportIntegrity";

describe("parseSourceChange", () => {
  it("reads a shifted segment, quotes and all", () => {
    expect(
      parseSourceChange(
        'www/data/Map001.json /events/1/pages/0/list/3/parameters/0: source changed since extraction; expected "「こんばんは」\\"旅\\"", found "Actor2"',
      ),
    ).toEqual({
      file: "www/data/Map001.json",
      pointer: "/events/1/pages/0/list/3/parameters/0",
      expected: '「こんばんは」"旅"',
      found: "Actor2",
    });
  });

  it("reads a location that no longer exists", () => {
    expect(
      parseSourceChange(
        'data/Map001.json /events/1/pages/0/list/4/parameters/0/0: source changed since extraction; expected "北の森へ", found nothing',
      ),
    ).toMatchObject({ expected: "北の森へ", found: null });
  });

  it("ignores every other integrity problem", () => {
    expect(
      parseSourceChange("data/System.json /gameTitle: unplanned text change"),
    ).toBeNull();
  });
});
