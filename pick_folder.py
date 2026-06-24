# pick_folder.py
# 폴더 선택창을 띄워 고른 경로를 stdout으로 출력한다. 취소 시 빈 줄.
import tkinter
from tkinter import filedialog


def main():
    root = tkinter.Tk()
    root.withdraw()
    root.attributes("-topmost", True)
    path = filedialog.askdirectory()
    root.destroy()
    print(path or "")


if __name__ == "__main__":
    main()
