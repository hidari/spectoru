// コールバックの渡し方のバリエーション。位置ではなく種類で本体を探す。

describe("オプション引数を挟むグループ", { concurrent: true }, () => {
  it("オプション引数を挟むテスト", { timeout: 1000 }, () => {});
});

describe("関数式で書かれたグループ", function () {
  it("関数式で書かれたテスト", async function () {});
});

describe("コールバックを持たないグループ");
