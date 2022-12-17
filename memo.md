# 開発メモ

## メモリ配置について

## ブートエントリ追加

https://qiita.com/deep_tkkn/items/b218e6a1d52a4d3b5e95

```
efibootmgr --create --disk /dev/sdX --part 1  --loader \\EFI\\htvmm\\htloader.efi --label htvmm #エントリー追加
efibootmgr -o 0003,0004,0000,0005,0001,0002 #OS のエントリーが優先されるように変更
efibootmgr -n $(efibootmgr | grep htvmm | cut -c 5-8) #htvmm を起動したい時のみ実行
```

## VM EntryとかVM Exitとかのモニタリング

https://resea.org/docs/servers/hv.html あたりをやる

## その他資料

- https://seiya.me/blog/implementing-hypervisor-on-resea