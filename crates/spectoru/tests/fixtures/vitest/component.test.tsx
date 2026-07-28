// JSX を含むテスト。TypeScript 文法では山括弧が型アサーションと衝突するため、
// 拡張子に応じて TSX 文法を選ぶ必要がある。

describe("ボタンコンポーネント", () => {
  it("ラベルが表示される", () => {
    const element = <button className="primary">送信</button>;
    expect(element).toBeTruthy();
  });
});
