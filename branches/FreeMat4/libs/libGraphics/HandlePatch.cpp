/*
 * Copyright (c) 2008 Samit Basu, Eugene Ingerman
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 *
 */
#include "HandlePatch.hpp"
#include "HandleAxis.hpp"
#include <qgl.h>

HandlePatch::HandlePatch() {
  ConstructProperties();
  SetupDefaults();
}

HandlePatch::~HandlePatch() {
}

QVector<double> HandlePatch::GetLimits() {
  QVector<double> limits;
  UpdateState();
  Array xdata(ArrayPropertyLookup("xdata"));
  Array ydata(ArrayPropertyLookup("ydata"));
  Array zdata(ArrayPropertyLookup("zdata"));
  Array cdata(ArrayPropertyLookup("cdata"));
  limits.push_back(ArrayMin(xdata));
  limits.push_back(ArrayMax(xdata));
  limits.push_back(ArrayMin(ydata));
  limits.push_back(ArrayMax(ydata));
  limits.push_back(ArrayMin(zdata));
  limits.push_back(ArrayMax(zdata));
  limits.push_back(ArrayMin(cdata));
  limits.push_back(ArrayMax(cdata));
  QVector<double> alphadata(VectorPropertyLookup("alphadata"));
  limits.push_back(VecMin(alphadata));
  limits.push_back(VecMax(alphadata));
  return limits;
}

void HandlePatch::ConstructProperties() {
  //!
  //@Module SURFACEPROPERTIES Surface Object Properties
  //@@Section HANDLE
  //@@Usage
  //Below is a summary of the properties for the axis.
  //\begin{itemize}
  //  \item @|alphadata| - @|vector| - This is a vector that
  // should contain as many elements as the surface data itself @|cdata|,
  // or a single scalar.  For a single scalar, all values of the surface
  // take on the same transparency.  Otherwise, the transparency of
  // each pixel is determined by the corresponding value from the @|alphadata|
  // vector.
  //  \item @|alphadatamapping| - @|{'scaled','direct','none'}| - For @|none|
  // mode (the default), no transparency is applied to the data.  For @|direct|
  // mode, the vector @|alphadata| contains values between @[0,M-1]| where
  // @|M| is the length of the alpha map stored in the figure.  For @|scaled|
  // mode, the @|alim| vector for the figure is used to linearly rescale the 
  // alpha data prior to lookup in the alpha map. 
  //  \item @|ambientstrength| - Not used.
  //  \item @|backfacelighting| - Not used.
  //  \item @|cdata| - @|array| - This is either a @|M x N| array or an 
  //  @|M x N x 3| array.  If the data is @|M x N| the surface is a scalar
  // surface (indexed mode), where the color associated with each surface pixel
  // is computed using the colormap and the @|cdatamapping| mode.  If the
  // data is @|M x N x 3| the surface is assumed to be in RGB mode, and the
  // colorpanes are taken directly from @|cdata| (the colormap is ignored).
  // Note that in this case, the data values must be between @[0,1]| for each
  // color channel and each point on the surface.
  //  \item @|cdatamapping| - @|{'scaled','direct'}| - For @|scaled| (the
  // default), the pixel values are scaled using the @|clim| vector for the
  // figure prior to looking up in the colormap.  For @|direct| mode, the
  // pixel values must be in the range @|[0,N-1| where @|N| is the number of
  // colors in the colormap.
  //  \item @|children| - Not used.
  //  \item @|diffusestrength| - Not used.
  //  \item @|edgealpha| - @|{'flat','interp','scalar'}| - Controls how the
  // transparency is mapped for the edges of the surface.
  //  \item @|edgecolor| - @|{'flat','interp','none',colorspec}| - Specifies
  // how the edges are colored.  For @|'flat'| the edges are flat colored,
  // meaning that the line segments that make up the edges are not shaded.
  // The color for the line is determined by the first edge point it is connected
  // to.
  //  \item @|edgelighting| - Not used.
  //  \item @|facealpha| - @|{'flat','interp','texturemap',scalar}| - Controls
  // how the transparency of the faces of the surface are controlled.  For
  // flat shading, the faces are constant transparency.  For interp mode, the faces
  // are smoothly transparently mapped.  If set to a scalar, all faces have the
  // same transparency.
  //  \item @|facecolor| - @|{'none','flat','interp',colorspec}| - Controls
  // how the faces are colored.  For @|'none'| the faces are uncolored, and
  // the surface appears as a mesh without hidden lines removed.  For @|'flat'|
  // the surface faces have a constant color.  For @|'interp'| smooth shading
  // is applied to the surface.  And if a colorspec is provided, then the
  // faces all have the same color.
  //  \item @|facelighting| - Not used.
  //  \item @|linestyle| - @|{'-','--',':','-.','none'}| - The style of the line used
  // to draw the edges.
  //  \item @|linewidth| - @|scalar| - The width of the line used to draw the edges.
  //  \item @|marker| - @|{'+','o','*','.','x','square','s','diamond','d','^','v','>','<'}| - 
  // The marker for data points on the line.  Some of these are redundant, as @|'square'| 
  // @|'s'| are synonyms, and @|'diamond'| and @|'d'| are also synonyms.
  //  \item @|markeredgecolor| - @|colorspec| - The color used to draw the marker.  For some
  // of the markers (circle, square, etc.) there are two colors used to draw the marker.
  // This property controls the edge color (which for unfilled markers) is the primary
  // color of the marker.
  //  \item @|markerfacecolor| - @|colorspec| - The color used to fill the marker.  For some
  // of the markers (circle, square, etc.) there are two colors used to fill the marker.
  //  \item @|markersize| - @|scalar| - Control the size of the marker.  Defaults to 6, which
  // is effectively the radius (in pixels) of the markers.
  //  \item @|meshstyle| - @|{'both','rows','cols}| - This property controls how the mesh is
  // drawn for the surface.  For @|rows| and @|cols| modes, only one set of edges is drawn.
  //  \item @|normalmode| - Not used.
  //  \item @|parent| - @|handle| - The axis containing the surface.
  //  \item @|specularcolorreflectance| - Not used.
  //  \item @|specularexponent| - Not used.
  //  \item @|specularstrength| - Not used.
  //  \item @|tag| - @|string| - You can set this to any string you want.
  //  \item @|type| - @|string| - Set to the string @|'surface'|.
  //  \item @|userdata| - @|array| - Available to store any variable you
  // want in the handle object.
  //  \item @|vertexnormals| - Not used.
  //  \item @|xdata| - @|array| - Must be a numeric array of size @|M x N| which contains
  // the x location of each point in the defined surface. Must be the same size as @|ydata|
  // and @|zdata|.  Alternately, you can specify an array of size @|1 x N| in which case
  // FreeMat replicates the vector to fill out an @|M x N| matrix.
  //  \item @|xdatamode| - @|{'auto','manual'}| - When set to @|auto| then FreeMat will
  // automatically generate the x coordinates.
  //  \item @|ydata| - @|array| - Must be a numeric array of size @|M x N| which contains
  // the y location of each point in the defined surface. Must be the same size as @|xdata|
  // and @|zdata|.   Alternately, you can specify an array of size @|M x 1| in which case
  // FreeMat replicates the vector to fill out an @|M x N| matrix.
  //  \item @|ydatamode| - @|{'auto','manual'}| - When set to @|auto| then FreeMat will
  // automatically generate the y coordinates.
  //  \item @|zdata| - @|array| - Must be a numeric array of size @|M x N| which contains
  // the y location of each point in the defined surface. Must be the same size as @|xdata|
  // and @|ydata|.
  //  \item @|visible| - @|{'on','off'}| - Controls whether the surface is
  // visible or not.
  //\end{itemize}
  //!
  AddProperty(new HPVector, "alphadata");
  AddProperty(new HPMappingMode, "alphadatamapping");
  AddProperty(new HPScalar,"ambientstrength");
  AddProperty(new HPBackFaceLighting,"backfacelighting");
  AddProperty(new HPArray, "cdata");
  AddProperty(new HPDataMappingMode, "cdatamapping");
  AddProperty(new HPAutoManual, "cdatamode");
  AddProperty(new HPHandles,"children");
  AddProperty(new HPScalar,"diffusestrength");
  AddProperty(new HPEdgeAlpha,"edgealpha");
  AddProperty(new HPColorInterp,"edgecolor");
  AddProperty(new HPLightingMode,"edgelighting");
  AddProperty(new HPFaceAlpha,"facealpha");
  AddProperty(new HPColorInterp,"facecolor");
  AddProperty(new HPLightingMode,"facelighting");
  AddProperty(new HPLineStyle,"linestyle");
  AddProperty(new HPScalar,"linewidth");
  AddProperty(new HPSymbol,"marker");
  AddProperty(new HPAutoFlatColor,"markeredgecolor");
  AddProperty(new HPAutoFlatColor,"markerfacecolor");
  AddProperty(new HPScalar,"markersize");
  AddProperty(new HPRowColumns,"meshstyle");
  AddProperty(new HPAutoManual,"normalmode");
  AddProperty(new HPHandles,"parent");
  AddProperty(new HPScalar,"specularcolorreflectance");
  AddProperty(new HPScalar,"specularexponent");
  AddProperty(new HPScalar,"specularstrength");
  AddProperty(new HPString,"tag");
  AddProperty(new HPString,"type");
  AddProperty(new HPArray,"userdata");
  AddProperty(new HPArray,"vertexnormals");
  AddProperty(new HPArray,"xdata");
  AddProperty(new HPAutoManual,"xdatamode");
  AddProperty(new HPArray,"ydata");
  AddProperty(new HPAutoManual,"ydatamode");
  AddProperty(new HPArray,"zdata");
  AddProperty(new HPOnOff,"visible");
}

void HandlePatch::SetupDefaults() {
  HPVector *hp = (HPVector*) LookupProperty("alphadata");
  QVector<double> gp;
  gp.push_back(1.0);
  hp->Data(gp);
  SetConstrainedStringDefault("alphadatamapping","none");
  SetScalarDefault("ambientstrength",0.55);
  SetConstrainedStringDefault("backfacelighting","unlit");
  SetConstrainedStringDefault("cdatamapping","scaled");
  SetConstrainedStringDefault("cdatamode","auto");
  SetScalarDefault("diffusestrength",0.6);
  SetScalarDefault("specularcolorreflectance",0.4);
  SetScalarDefault("specularexponent",0.1);
  SetScalarDefault("specularstrength",0.5);
  SetStringDefault("type","image");
  SetConstrainedStringDefault("visible","on");
  SetConstrainedStringScalarDefault("edgealpha","scalar",1);
  SetConstrainedStringColorDefault("edgecolor","colorspec",0,0,0);
  SetConstrainedStringDefault("edgelighting","none");
  SetConstrainedStringScalarDefault("facealpha","scalar",1);
  SetConstrainedStringDefault("facecolor","flat");
  SetConstrainedStringDefault("facelighting","none");
  SetConstrainedStringDefault("linestyle","-");
  SetScalarDefault("linewidth",0.5);
  SetConstrainedStringDefault("marker","none");
  SetConstrainedStringColorDefault("markeredgecolor","auto",0,0,0);
  SetConstrainedStringColorDefault("markerfacecolor","none",0,0,0);
  SetScalarDefault("markersize",6);
  SetConstrainedStringDefault("meshstyle","both");
  SetConstrainedStringDefault("normalmode","auto");
  SetStringDefault("type","patch");
  SetStringDefault("xdatamode","auto");
  SetStringDefault("ydatamode","auto");
}

void HandlePatch::DoAutoXMode() {
  Array zdata(ArrayPropertyLookup("zdata"));
  Array xdata(Array::doubleMatrixConstructor(zdata.rows(),zdata.columns()));
  double *dp = (double*) xdata.getReadWriteDataPointer();
  int cols(zdata.columns());
  int rows(zdata.rows());
  for (int j=0;j<cols;j++)
    for (int i=0;i<rows;i++)
      dp[i+j*rows] = j+1;
  HPArray *hp = (HPArray*) LookupProperty("xdata");
  hp->Data(xdata);
}

void HandlePatch::DoAutoYMode() {
  Array zdata(ArrayPropertyLookup("zdata"));
  Array ydata(Array::doubleMatrixConstructor(zdata.rows(),zdata.columns()));
  double *dp = (double*) ydata.getReadWriteDataPointer();
  int cols(zdata.columns());
  int rows(zdata.rows());
  for (int j=0;j<cols;j++)
    for (int i=0;i<rows;i++)
      dp[i+j*rows] = i+1;
  HPArray *hp = (HPArray*) LookupProperty("ydata");
  hp->Data(ydata);
}

void HandlePatch::DoAutoCMode() {
  Array zdata(ArrayPropertyLookup("zdata"));
  HPArray *hp = (HPArray*) LookupProperty("cdata");
  hp->Data(zdata);
}

void HandlePatch::UpdateState() {
  if (HasChanged("xdata"))
    ToManual("xdatamode");
  if (HasChanged("ydata"))
    ToManual("ydatamode");
  if (HasChanged("cdata"))
    ToManual("cdatamode");
  if (IsAuto("xdatamode")) 
    DoAutoXMode();
  if (IsAuto("ydatamode"))
    DoAutoYMode();
  if (IsAuto("cdatamode"))
    DoAutoCMode();
  HandleImage::UpdateCAlphaData();
}



void HandlePatch::PaintMe(RenderEngine& gc) {
  UpdateState();
  if (StringCheck("visible","off"))
    return;
  // Get the x,y,z & color data points
  Array xdata(ArrayPropertyLookup("xdata"));
  xdata.promoteType(FM_DOUBLE);
  Array ydata(ArrayPropertyLookup("ydata"));
  ydata.promoteType(FM_DOUBLE);
  Array zdata(ArrayPropertyLookup("zdata"));
  zdata.promoteType(FM_DOUBLE);
  if (zdata.isEmpty()) return;
  // There are many render styles...
  // edgealpha, edgecolor, facealpha, facecolor
  // facecolor
  // Texture mapping not supported yet
  if (StringCheck("facecolor","texturemap")) return;
  if (StringCheck("facealpha","texturemap")) return;
  // A quadstrip is defined by its 
  QVector<QVector<cpoint> > surfquads(BuildQuadsNoTexMap((HPConstrainedStringColor*) 
								 LookupProperty("facecolor"),
								 (HPConstrainedStringScalar*)
								 LookupProperty("facealpha")));
  QVector<QVector<cpoint> > edgequads(BuildQuadsNoTexMap((HPConstrainedStringColor*) 
								 LookupProperty("edgecolor"),
								 (HPConstrainedStringScalar*)
								 LookupProperty("edgealpha")));

  HandleAxis *ax = GetParentAxis();
  if( ax->StringCheck("nextplot","add") ){
      Saved_surfquads += surfquads;
      Saved_edgequads += edgequads;
  }
  else{
      Saved_surfquads = surfquads;
      Saved_edgequads = edgequads;
  }
  gc.quadStrips(Saved_surfquads,StringCheck("facecolor","flat"),
		Saved_edgequads,StringCheck("edgecolor","flat"));
}

void HandlePatch::AxisPaintingDone( void )
{
    Saved_surfquads.clear();
    Saved_edgequads.clear();
}
