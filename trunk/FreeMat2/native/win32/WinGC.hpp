#ifndef __WinGC_hpp__
#define __WinGC_hpp__

#include <windows.h>
#include "../../libs/libXP/GraphicsContext.hpp"

class WinGC : public GraphicsContext {
  HDC hdc;
  int m_width;
  int m_height;
  std::string m_fontname;
  int m_fontsize;
  std::vector<Rect2D> clipstack;
  HFONT m_hfont;
  HFONT m_vfont;

  HRGN clipwin;
  Color bgcol, fgcol;
  LineStyleType m_style;
  bool colormapActive;
  HPALETTE m_colormap;
public:
  WinGC(HDC dc, int width, int height);
  ~WinGC();
  virtual Point2D GetCanvasSize();
  virtual Point2D GetTextExtent(std::string label);
  virtual void DrawTextString(std::string label, Point2D pos, OrientationType orient = ORIENT_0);
  virtual void SetFont(int fontsize);
  virtual Color SetBackGroundColor(Color col);
  virtual Color SetForeGroundColor(Color col);
  virtual LineStyleType SetLineStyle(LineStyleType style);
  virtual void DrawLine(Point2D pos1, Point2D pos2);
  virtual void DrawPoint(Point2D pos);
  virtual void DrawCircle(Point2D pos, int radius);
  virtual void DrawRectangle(Rect2D rect);
  virtual void FillRectangle(Rect2D rect);
  virtual void FillQuad(Point2D p1, Point2D p2, Point2D p3, Point2D p4);
  virtual void DrawQuad(Point2D p1, Point2D p2, Point2D p3, Point2D p4);
  virtual void DrawLines(std::vector<Point2D> pts);
  virtual void PushClippingRegion(Rect2D rect);
  virtual Rect2D PopClippingRegion();
  virtual void BlitImage(unsigned char *data, int width, int height, int x0, int y0);
#if 0
  virtual bool IsColormapActive();
  virtual HPALETTE GetColormap();
private:	
  virtual void BlitImagePseudoColor(unsigned char *data, int width, int height, int x0, int y0);
#endif
};

#endif